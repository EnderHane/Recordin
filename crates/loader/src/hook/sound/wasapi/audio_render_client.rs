use std::sync::{
    Arc,
    atomic::{
        AtomicBool,
        AtomicU64,
        Ordering,
    },
};

use windows::Win32::{
    Foundation::S_OK,
    Media::Audio::{
        AUDCLNT_BUFFERFLAGS_SILENT,
        AUDCLNT_E_BUFFER_TOO_LARGE,
        AUDCLNT_E_INVALID_SIZE,
        AUDCLNT_E_OUT_OF_ORDER,
        IAudioRenderClient,
        IAudioRenderClient_Impl,
    },
};
use windows_core::implement;

use crate::{
    hook::sound::wasapi::audio_client::MyAudioClient,
    output::{
        audio_codec,
        audio_codec::AudioEncDuplex,
    },
};

#[implement(IAudioRenderClient)]
pub(super) struct MyAudioRenderClient {
    buf: Box<[u8]>,
    counter: Arc<AtomicU64>,
    requested: AtomicBool,
    frame_req: AtomicU64,
    encoder: Option<AudioEncDuplex>,
}

impl MyAudioRenderClient {
    pub(super) fn new(buffer_size: usize, counter: Arc<AtomicU64>) -> Self {
        let buf = vec![0; buffer_size * 2 * 4].into_boxed_slice();
        let requested = AtomicBool::new(false);
        let frame_req = AtomicU64::new(0);
        let num = audio_codec::STREAM_COUNTER.fetch_add(1, Ordering::Relaxed);
        let encoder = audio_codec::create_encoder(num);
        Self {
            buf,
            counter,
            requested,
            frame_req,
            encoder,
        }
    }

    fn on_release(&self, len: usize, muted: bool) -> Option<()> {
        let (tx, rx) = self.encoder.as_ref()?;
        let mut buf = rx.recv().ok()?;
        buf.resize(len * 4 * 2, 0);
        if !muted {
            buf.copy_from_slice(&self.buf[0..len * 4 * 2])
        }
        tx.send(buf).ok()?;
        Some(())
    }
}

#[allow(non_snake_case)]
impl IAudioRenderClient_Impl for MyAudioRenderClient_Impl {
    fn GetBuffer(&self, frames_req: u32) -> windows_result::Result<*mut u8> {
        // log::trace!("MyAudioRenderClient GetBuffer");
        // log::trace!("num_frames_requested: {}", num_frames_requested);
        if frames_req == 0 {
            Err(S_OK)?
        }
        if self.requested.load(Ordering::Acquire) {
            Err(AUDCLNT_E_OUT_OF_ORDER)?
        }
        self.requested.store(true, Ordering::Release);
        if (frames_req * MyAudioClient::CHANNELS) as usize > self.buf.len() {
            Err(AUDCLNT_E_BUFFER_TOO_LARGE)?
        }
        self.frame_req.store(frames_req as _, Ordering::Relaxed);
        Ok(self.buf.as_ptr().cast_mut())
    }

    fn ReleaseBuffer(&self, written: u32, flags: u32) -> windows_result::Result<()> {
        // log::trace!("MyAudioRenderClient ReleaseBuffer");
        // log::trace!("num_frames_written: {}", num_frames_written);
        if written == 0 {
            Err(S_OK)?
        }
        if !self.requested.load(Ordering::Acquire) {
            Err(AUDCLNT_E_OUT_OF_ORDER)?;
        }
        if written as u64 > self.frame_req.load(Ordering::Relaxed) {
            Err(AUDCLNT_E_INVALID_SIZE)?
        }
        let len = self.frame_req.load(Ordering::Relaxed) as usize;
        let muted = flags as i32 & AUDCLNT_BUFFERFLAGS_SILENT.0 != 0;
        self.on_release(len, muted);
        let f = self.frame_req.load(Ordering::Relaxed);
        self.counter.fetch_add(f, Ordering::Relaxed);
        self.frame_req.store(0, Ordering::Relaxed);
        self.requested.store(false, Ordering::Release);
        Ok(())
    }
}
