use std::{
    cell::LazyCell,
    collections::BTreeMap,
    fs::File,
    num::NonZero,
    sync::atomic::{
        AtomicU32,
        Ordering,
    },
};

use scuffle_ffmpeg::{
    AVPixelFormat,
    codec::EncoderCodec,
    dict::Dictionary,
    encoder::{
        Encoder,
        VideoEncoderSettings,
    },
    frame::VideoFrame,
    io::{
        Output,
        OutputOptions,
    },
    rational::Rational,
    scaler::VideoScaler,
};

use crate::env;

pub(crate) static SURFACE_COUNTER: AtomicU32 = AtomicU32::new(0);

pub(crate) type EncDuplex = (kanal::Sender<Vec<[u8; 3]>>, kanal::Receiver<Vec<[u8; 3]>>);

pub(crate) fn create_encoder(width: usize, height: usize) -> Option<EncDuplex> {
    env::should_emit_video().then_some({})?;
    let path = env::VIDEO_OUTPUT.as_ref()?.clone();
    let (tx1, rx1) = kanal::bounded(30);
    let (tx2, rx2) = kanal::bounded(30);
    for _ in 0..10 {
        tx2.send(Vec::with_capacity(width * height)).unwrap();
    }
    std::thread::spawn(move || {
        match loop_encode(width, height, rx1, tx2, move || {
            let mut new_path = path.clone();
            let mut stem = path
                .file_stem()
                .ok_or(anyhow::anyhow!("must have a file name"))?
                .to_owned();
            let c = SURFACE_COUNTER.fetch_add(1, Ordering::Relaxed);
            if c != 0 {
                stem.push(c.to_string());
            }
            new_path.set_file_name(stem);
            if let Some(ext) = path.extension() {
                new_path.set_extension(ext);
            }
            log::trace!("Video output: \n{:?}", path);
            Ok(File::create(path)?)
        }) {
            Ok(_) => {
                log::info!("Video encoder successfully completed");
            }
            Err(e) => {
                log::warn!("Video encoder error: {}", e);
            }
        }
    });
    Some((tx1, rx2))
}

fn loop_encode(
    width: usize,
    height: usize,
    rx: kanal::Receiver<Vec<[u8; 3]>>,
    tx: kanal::Sender<Vec<[u8; 3]>>,
    lazy_file: impl FnOnce() -> anyhow::Result<File> + 'static,
) -> anyhow::Result<()> {
    let fps = env::FPS.get();
    let encode_codec_name = env::VIDEO_ENCODER
        .get()
        .ok_or(anyhow::anyhow!("Video encoder not set"))?;
    log::trace!("Video encoder: {}", encode_codec_name);
    let args = &env::VIDEO_ARGS.as_ref().unwrap_or_else(|| {
        static EMPTY: BTreeMap<String, String> = BTreeMap::new();
        &EMPTY
    });
    struct LazyGroup {
        output: Output<File>,
        encoder: Encoder,
        frame: VideoFrame,
        scaler: VideoScaler,
    }
    let mut lazy_group = LazyCell::new(|| {
        let writer = lazy_file()?;
        let mut output = Output::seekable(
            writer,
            OutputOptions::builder().format_name("Matroska")?.build(),
        )?;
        let codec = EncoderCodec::by_name(&encode_codec_name)
            .ok_or(anyhow::anyhow!("encoder {} not found", encode_codec_name))?;
        let dict = Dictionary::try_from_iter(args.iter().map(|(k, v)| (k.as_str(), v.as_str())))?;
        let approx_fps = num_rational::Ratio::approximate_float(fps).unwrap();
        let approx_tbn = approx_fps.recip();
        let video_settings = VideoEncoderSettings::builder()
            .width(width as _)
            .height(height as _)
            .pixel_format(AVPixelFormat::Yuv420p)
            .frame_rate(Rational::new(
                *approx_fps.numer(),
                NonZero::new(*approx_fps.denom()).unwrap(),
            ))
            .codec_specific_options(dict)
            .build();
        let encoder = Encoder::new(
            codec,
            &mut output,
            Rational::new(
                *approx_tbn.numer(),
                NonZero::new(*approx_tbn.denom()).unwrap(),
            ),
            Rational::new(1, NonZero::new(1000).unwrap()),
            video_settings,
        )?;
        output.write_header()?;
        let frame = VideoFrame::builder()
            .width(width as _)
            .height(height as _)
            .pix_fmt(AVPixelFormat::Rgb24)
            .time_base(encoder.incoming_time_base())
            .build()?;
        let scaler = VideoScaler::new(
            width as _,
            height as _,
            AVPixelFormat::Rgb24,
            width as _,
            height as _,
            AVPixelFormat::Yuv420p,
        )?;
        let lazy = LazyGroup {
            output,
            encoder,
            frame,
            scaler,
        };
        anyhow::Ok(lazy)
    });
    let mut count = 0;
    while let Ok(buf) = rx.recv() {
        let LazyGroup {
            output: out,
            encoder: e,
            frame: fr,
            scaler: sc,
        } = lazy_group.as_mut().map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut fr_data = fr.data_mut(0).unwrap();
        for (h, row_in) in buf.chunks_exact(width).enumerate() {
            let line = fr_data.get_row_mut(h).unwrap();
            line.copy_from_slice(row_in.as_flattened());
        }
        tx.send(buf).ok();
        fr.set_pts(Some(count));
        count += 1;
        let yuv = sc.process(fr)?;
        e.send_frame(yuv)?;
        while let Some(packet) = e.receive_packet()? {
            out.write_interleaved_packet(packet)?;
        }
    }
    if count > 0
        && let Ok(LazyGroup {
            output: out,
            encoder: e,
            ..
        }) = lazy_group.as_mut()
    {
        e.send_eof()?;
        while let Some(packet) = e.receive_packet()? {
            out.write_interleaved_packet(packet)?;
        }
        out.write_trailer()?;
    }
    Ok(())
}
