use std::{
    collections::BTreeMap,
    fs::File,
    num::NonZero,
    sync::atomic::AtomicU32,
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

pub(crate) fn create_encoder(stream_number: u32, width: usize, height: usize) -> Option<EncDuplex> {
    env::should_emit_video().then_some({})?;
    let (tx1, rx1) = kanal::bounded(30);
    let (tx2, rx2) = kanal::bounded(30);
    for _ in 0..10 {
        tx2.try_send(Vec::with_capacity(width * height)).ok();
    }
    let filename = env::VIDEO_OUTPUT
        .as_ref()?
        .replace("{n}", &stream_number.to_string());
    std::thread::spawn(
        move || match loop_encode(width, height, rx1, tx2, filename) {
            Ok(_) => {}
            Err(e) => {
                log::info!("Video encoder not run: {}", e);
            }
        },
    );
    Some((tx1, rx2))
}

fn loop_encode(
    width: usize,
    height: usize,
    rx: kanal::Receiver<Vec<[u8; 3]>>,
    tx: kanal::Sender<Vec<[u8; 3]>>,
    filename: String,
) -> anyhow::Result<()> {
    let fps = env::FPS.get().ok_or(anyhow::anyhow!("FPS not set"))?;
    let encode_codec_name = env::VIDEO_ENCODER
        .get()
        .ok_or(anyhow::anyhow!("Video encoder not set"))?;
    log::trace!("Video encoder: {}", encode_codec_name);
    let args = &env::VIDEO_ARGS.as_ref().unwrap_or_else(|| {
        static EMPTY: BTreeMap<String, String> = BTreeMap::new();
        &EMPTY
    });
    let mut output = lazycell::LazyCell::new();
    let mut encoder = lazycell::LazyCell::new();
    let mut frame = lazycell::LazyCell::new();
    let mut scaler = lazycell::LazyCell::new();
    let mut count = 0;
    while let Ok(buf) = rx.recv() {
        let out = output.try_borrow_mut_with(|| {
            log::trace!("Video output to:\n{}", filename);
            let writer = File::create(&filename)?;
            let output = Output::new(
                writer,
                OutputOptions::builder().format_name("Matroska")?.build(),
            )?;
            anyhow::Ok(output)
        })?;
        let v_enc = encoder.try_borrow_mut_with(|| {
            let codec = EncoderCodec::by_name(&encode_codec_name)
                .ok_or(anyhow::anyhow!("encoder {} not found", encode_codec_name))?;
            let dict =
                Dictionary::try_from_iter(args.iter().map(|(k, v)| (k.as_str(), v.as_str())))?;
            log::trace!("Video codec options:\n{:?}", dict);
            let aprox_fps = num_rational::Ratio::approximate_float(fps).unwrap();
            let aprox_tbn = aprox_fps.recip();
            let video_settings = VideoEncoderSettings::builder()
                .width(width as _)
                .height(height as _)
                .pixel_format(AVPixelFormat::Yuv420p)
                .frame_rate(Rational::new(
                    *aprox_fps.numer(),
                    NonZero::new(*aprox_fps.denom()).unwrap(),
                ))
                .codec_specific_options(dict)
                .build();
            let enc = Encoder::new(
                codec,
                out,
                Rational::new(
                    *aprox_tbn.numer(),
                    NonZero::new(*aprox_tbn.denom()).unwrap(),
                ),
                Rational::new(1, NonZero::new(1000).unwrap()),
                video_settings,
            )?;
            out.write_header()?;
            anyhow::Ok(enc)
        })?;
        let fr = frame.try_borrow_mut_with(|| {
            VideoFrame::builder()
                .width(width as _)
                .height(height as _)
                .pix_fmt(AVPixelFormat::Rgb24)
                .time_base(v_enc.incoming_time_base())
                .build()
        })?;
        let scale = scaler.try_borrow_mut_with(|| {
            VideoScaler::new(
                width as _,
                height as _,
                AVPixelFormat::Rgb24,
                width as _,
                height as _,
                AVPixelFormat::Yuv420p,
            )
        })?;
        let mut fr_data = fr.data_mut(0).unwrap();
        for (h, row_in) in buf.chunks_exact(width).enumerate() {
            let line = fr_data.get_row_mut(h).unwrap();
            line.copy_from_slice(row_in.as_flattened());
        }
        tx.send(buf).ok();
        fr.set_pts(Some(count));
        count += 1;
        let yuv = scale.process(fr)?;
        v_enc.send_frame(yuv)?;
        while let Some(packet) = v_enc.receive_packet()? {
            out.write_interleaved_packet(packet)?;
        }
    }
    if let Some((v_enc, out)) = encoder.borrow_mut().zip(output.borrow_mut()) {
        v_enc.send_eof()?;
        while let Some(packet) = v_enc.receive_packet()? {
            out.write_interleaved_packet(packet)?;
        }
        out.write_trailer()?;
    }
    Ok(())
}
