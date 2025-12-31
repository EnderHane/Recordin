use std::{
    fs::File,
    num::NonZero,
    sync::atomic::AtomicU32,
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

pub(super) static STREAM_COUNTER: AtomicU32 = AtomicU32::new(0);

pub(super) type AudioEncDuplex = (kanal::Sender<Vec<u8>>, kanal::Receiver<Vec<u8>>);

pub(super) fn create_encoder(stream_number: u32) -> Option<AudioEncDuplex> {
    let (tx1, rx1) = kanal::bounded(30);
    let (tx2, rx2) = kanal::bounded(30);
    for _ in 0..10 {
        tx2.try_send(Vec::new()).ok();
    }
    let filename = env::AUDIO_OUTPUT
        .as_ref()?
        .replace("{n}", &stream_number.to_string());
    std::thread::spawn(move || match loop_encode(rx1, tx2, filename) {
        Ok(_) => {}
        Err(e) => {
            log::info!("Audio encoder not run: {}", e);
        }
    });
    (tx1, rx2).into()
}

fn loop_encode(
    rx: kanal::Receiver<Vec<u8>>,
    tx: kanal::Sender<Vec<u8>>,
    filename: String,
) -> anyhow::Result<()> {
    log::trace!("Audio encoder: {}", "flac");
    let mut output = lazycell::LazyCell::new();
    let mut encoder = lazycell::LazyCell::new();
    let mut frame = lazycell::LazyCell::new();
    let mut resampler = lazycell::LazyCell::new();
    let mut count = 0;
    let mut ring = ExpSliceRB::with_capacity(NonZero::new(4608 * 4 * 2).unwrap());
    while let Ok(buf) = rx.recv() {
        let out = output.try_borrow_mut_with(|| {
            log::trace!("Audio output to:\n{}", filename);
            let writer = File::create(&filename)?;
            let output = Output::new(
                writer,
                OutputOptions::builder().format_name("flac")?.build(),
            )?;
            anyhow::Ok(output)
        })?;
        let a_enc = encoder.try_borrow_mut_with(|| {
            let codec =
                EncoderCodec::by_name("flac").ok_or(anyhow::anyhow!("encoder flac not found"))?;
            let audio_settings = AudioEncoderSettings::builder()
                .sample_rate(48000)
                .ch_layout(AudioChannelLayout::new(2).unwrap())
                .sample_fmt(AVSampleFormat::S16)
                .build();
            let enc = Encoder::new(
                codec,
                out,
                Rational::new(1, NonZero::new(48000).unwrap()),
                Rational::new(1, NonZero::new(1000).unwrap()),
                audio_settings,
            )?;
            out.write_header()?;
            anyhow::Ok(enc)
        })?;
        let fr = frame.try_borrow_mut_with(|| {
            AudioFrame::builder()
                .channel_layout(AudioChannelLayout::new(2).unwrap())
                .nb_samples(4608)
                .sample_fmt(AVSampleFormat::Flt)
                .sample_rate(48000)
                .build()
        })?;
        let resample = resampler.try_borrow_mut_with(|| {
            Resampler::new(
                AudioChannelLayout::new(2).unwrap(),
                AVSampleFormat::Flt,
                48000,
                AudioChannelLayout::new(2).unwrap(),
                AVSampleFormat::S16,
                48000,
            )
        })?;
        ring.write(&buf);
        tx.send(buf).ok();
        while ring.len() >= 4608 * 4 * 2 {
            let fr_data = fr.data_mut(0).unwrap();
            ring.read_into(&mut fr_data[0..4608 * 4 * 2]);
            let mut s16 = resample.process(fr)?;
            s16.set_pts(Some(count));
            count += 4608;
            a_enc.send_frame(&s16)?;
            while let Some(packet) = a_enc.receive_packet()? {
                out.write_interleaved_packet(packet)?;
            }
        }
    }
    if let Some(a_enc) = encoder.borrow_mut()
        && let Some(out) = output.borrow_mut()
        && let Some(resample) = resampler.borrow_mut()
        && let Some(fr) = frame.borrow_mut()
    {
        let rest = ring.len();
        let fr_data = fr.data_mut(0).unwrap();
        ring.read_into(&mut fr_data[0..rest]);
        fr_data[rest..4608 * 4 * 2].fill(0);
        let mut s16 = resample.process(fr)?;
        s16.set_pts(Some(count));
        count += rest as i64 / 4 / 2;
        a_enc.send_frame(&s16)?;
        a_enc.send_eof()?;
        while let Some(packet) = a_enc.receive_packet()? {
            out.write_interleaved_packet(packet)?;
        }
        out.write_trailer()?;
    }
    log::trace!("Audio encoded samples {count}");
    Ok(())
}
