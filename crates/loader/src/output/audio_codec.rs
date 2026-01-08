use std::{
    cell::LazyCell,
    fs::File,
    num::NonZero,
    sync::atomic::{
        AtomicU32,
        Ordering,
    },
};

use expanding_slice_rb::ExpSliceRB;
use scuffle_ffmpeg::{
    AVSampleFormat,
    codec::EncoderCodec,
    encoder::{
        AudioEncoderSettings,
        Encoder,
    },
    frame::{
        AudioChannelLayout,
        AudioFrame,
    },
    io::{
        Output,
        OutputOptions,
    },
    rational::Rational,
    resampler::Resampler,
};

use crate::env;

pub(crate) static STREAM_COUNTER: AtomicU32 = AtomicU32::new(0);

pub(crate) type AudioEncDuplex = (kanal::Sender<Vec<u8>>, kanal::Receiver<Vec<u8>>);

pub(crate) fn create_encoder() -> Option<AudioEncDuplex> {
    let (tx1, rx1) = kanal::bounded(30);
    let (tx2, rx2) = kanal::bounded(30);
    for _ in 0..10 {
        tx2.try_send(Vec::new()).ok();
    }
    let path = env::AUDIO_OUTPUT.as_ref()?.clone();
    std::thread::spawn(move || {
        match loop_encode(rx1, tx2, move || {
            let mut new_path = path.clone();
            let mut stem = path
                .file_stem()
                .ok_or(anyhow::anyhow!("must have a file name"))?
                .to_owned();
            let c = STREAM_COUNTER.fetch_add(1, Ordering::Relaxed);
            if c != 0 {
                stem.push(c.to_string());
            }
            new_path.set_file_name(stem);
            if let Some(ext) = path.extension() {
                new_path.set_extension(ext);
            }
            log::trace!("Audio output: \n{:?}", path);
            Ok(File::create(path)?)
        }) {
            Ok(_) => {
                log::info!("Audio encoder successfully completed");
            }
            Err(e) => {
                log::warn!("Audio encoder error: {}", e);
            }
        }
    });
    (tx1, rx2).into()
}

fn loop_encode(
    rx: kanal::Receiver<Vec<u8>>,
    tx: kanal::Sender<Vec<u8>>,
    lazy_file: impl FnOnce() -> anyhow::Result<File> + 'static,
) -> anyhow::Result<()> {
    log::trace!("Audio encoder: {}", "wavpack");
    struct LazyGroup {
        output: Output<File>,
        encoder: Encoder,
        frame: AudioFrame,
        resampler: Resampler,
    }
    let mut lazy_group = LazyCell::new(|| {
        let writer = lazy_file()?;
        let mut output = Output::seekable(
            writer,
            OutputOptions::builder().format_name("Matroska")?.build(),
        )?;
        let codec = EncoderCodec::by_name("wavpack").unwrap();
        let audio_settings = AudioEncoderSettings::builder()
            .sample_rate(48000)
            .ch_layout(AudioChannelLayout::new(2).unwrap())
            .sample_fmt(AVSampleFormat::Fltp)
            .build();
        let encoder = Encoder::new(
            codec,
            &mut output,
            Rational::new(1, NonZero::new(48000).unwrap()),
            Rational::new(1, NonZero::new(1000).unwrap()),
            audio_settings,
        )?;
        output.write_header()?;
        let frame = AudioFrame::builder()
            .channel_layout(AudioChannelLayout::new(2).unwrap())
            .nb_samples(24000)
            .sample_fmt(AVSampleFormat::Flt)
            .sample_rate(48000)
            .build()?;
        let resampler = Resampler::new(
            AudioChannelLayout::new(2).unwrap(),
            AVSampleFormat::Flt,
            48000,
            AudioChannelLayout::new(2).unwrap(),
            AVSampleFormat::Fltp,
            48000,
        )?;
        let lazy = LazyGroup {
            output,
            encoder,
            frame,
            resampler,
        };
        anyhow::Ok(lazy)
    });
    let mut count = 0;
    let mut ring = ExpSliceRB::with_capacity(NonZero::new(24000 * 4 * 2).unwrap());
    while let Ok(buf) = rx.recv() {
        let LazyGroup {
            output: o,
            encoder: e,
            frame: fr,
            resampler: re,
        } = lazy_group.as_mut().map_err(|e| anyhow::anyhow!("{e}"))?;
        ring.write(&buf);
        tx.send(buf).ok();
        while ring.len() >= 24000 * 4 * 2 {
            let fr_data = fr.data_mut(0).unwrap();
            ring.read_into(&mut fr_data[0..24000 * 4 * 2]);
            let mut rsp = re.process(fr)?;
            rsp.set_pts(count.into());
            count += 24000;
            e.send_frame(&rsp)?;
            while let Some(packet) = e.receive_packet()? {
                o.write_interleaved_packet(packet)?;
            }
        }
    }
    if count > 0
        && let Ok(LazyGroup {
            output: o,
            encoder: e,
            frame: _,
            resampler: re,
        }) = lazy_group.as_mut()
    {
        let rest = ring.len();
        let mut fr = AudioFrame::builder()
            .nb_samples(rest as i32 / 4 / 2)
            .channel_layout(AudioChannelLayout::new(2).unwrap())
            .sample_fmt(AVSampleFormat::Flt)
            .sample_rate(48000)
            .build()?;
        let fr_data = fr.data_mut(0).unwrap();
        ring.read_into(&mut fr_data[0..rest]);
        let mut rsp = re.process(&fr)?;
        rsp.set_pts(count.into());
        count += rest as i64 / 4 / 2;
        e.send_frame(&rsp)?;
        while let Some(pk) = e.receive_packet()? {
            o.write_interleaved_packet(pk)?;
        }
        e.send_eof()?;
        while let Some(pk) = e.receive_packet()? {
            o.write_interleaved_packet(pk)?;
        }
        o.write_trailer()?;
        log::trace!("Audio encoded samples {count}");
    }
    Ok(())
}
