use std::{
    mem,
    sync::{
        Arc,
        OnceLock,
        atomic::{
            AtomicU64,
            Ordering,
        },
    },
};

use parking_lot::Mutex;
use windows::Win32::{
    Foundation::{
        E_NOINTERFACE,
        E_POINTER,
        HANDLE,
        S_FALSE,
        S_OK,
    },
    Media::{
        Audio::{
            AUDCLNT_E_ALREADY_INITIALIZED,
            AUDCLNT_E_EVENTHANDLE_NOT_EXPECTED,
            AUDCLNT_E_EVENTHANDLE_NOT_SET,
            AUDCLNT_E_EXCLUSIVE_MODE_NOT_ALLOWED,
            AUDCLNT_E_NOT_INITIALIZED,
            AUDCLNT_E_NOT_STOPPED,
            AUDCLNT_E_UNSUPPORTED_FORMAT,
            AUDCLNT_SHAREMODE,
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
            IAudioClient,
            IAudioClient_Impl,
            IAudioRenderClient,
            WAVEFORMATEX,
            WAVEFORMATEXTENSIBLE,
            WAVEFORMATEXTENSIBLE_0,
        },
        KernelStreaming::WAVE_FORMAT_EXTENSIBLE,
        Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
    },
    System::Com::CoTaskMemAlloc,
};
use windows_core::{
    GUID,
    Interface,
    implement,
};
use windows_result::HRESULT;

use crate::hook::{
    sound,
    sound::{
        EVENTS,
        wasapi::audio_render_client::MyAudioRenderClient,
    },
    timing,
};

#[implement(IAudioClient)]
pub(in crate::hook::sound) struct MyAudioClient {
    init: OnceLock<InitializedState>,
    wave_format: WAVEFORMATEXTENSIBLE,
}

struct InitializedState {
    buffer_size: usize,
    event_handle: Option<OnceLock<Arc<usize>>>,
    frame_counter: Arc<AtomicU64>,
    start: Mutex<Option<StartState>>,
}

struct StartState {
    time: i64,
}

impl MyAudioClient {
    const SAMPLE_RATE: u32 = 48000;
    pub(in crate::hook::sound) const CHANNELS: u32 = 2;
    const SIZE_OF_SAMPLE: u32 = size_of::<f32>() as u32;

    pub fn new() -> Self {
        let init = OnceLock::new();
        let wave_format = WAVEFORMATEXTENSIBLE {
            Format: WAVEFORMATEX {
                wFormatTag: WAVE_FORMAT_EXTENSIBLE as u16,
                nChannels: Self::CHANNELS as _,
                nSamplesPerSec: Self::SAMPLE_RATE,
                nAvgBytesPerSec: Self::SAMPLE_RATE * Self::SIZE_OF_SAMPLE * Self::CHANNELS,
                nBlockAlign: (Self::CHANNELS * Self::SIZE_OF_SAMPLE) as _,
                wBitsPerSample: (Self::SIZE_OF_SAMPLE * 8) as _,
                cbSize: 22,
            },
            Samples: WAVEFORMATEXTENSIBLE_0 {
                wValidBitsPerSample: 32,
            },
            dwChannelMask: 0b11,
            SubFormat: KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
        };
        Self { init, wave_format }
    }
}

#[allow(non_snake_case)]
impl IAudioClient_Impl for MyAudioClient_Impl {
    fn Initialize(
        &self,
        share_mode: AUDCLNT_SHAREMODE,
        stream_flags: u32,
        hns_buffer_duration: i64,
        _hns_periodicity: i64,
        format: *const WAVEFORMATEX,
        _audio_session_guid: *const GUID,
    ) -> windows_result::Result<()> {
        log::trace!("MyAudioClient Initialize");
        log::trace!("Buffer duration: {}", hns_buffer_duration);
        log::trace!("Periodicity: {}", _hns_periodicity);
        if self.init.get().is_some() {
            Err(AUDCLNT_E_ALREADY_INITIALIZED)?
        }
        if share_mode != AUDCLNT_SHAREMODE_SHARED {
            Err(AUDCLNT_E_EXCLUSIVE_MODE_NOT_ALLOWED)?
        }
        if format.is_null() {
            Err(E_POINTER)?
        }
        let format_ex = unsafe { *format };
        if format_ex.wFormatTag != WAVE_FORMAT_EXTENSIBLE as u16 || format_ex.cbSize < 22 {
            Err(AUDCLNT_E_UNSUPPORTED_FORMAT)?
        }
        let format_ext: WAVEFORMATEXTENSIBLE = unsafe { *format.cast() };
        let t_format_ext: [u8; size_of::<WAVEFORMATEXTENSIBLE>()] =
            unsafe { mem::transmute(format_ext) };
        let my_format_ext: [u8; size_of::<WAVEFORMATEXTENSIBLE>()] =
            unsafe { mem::transmute(self.wave_format) };
        if t_format_ext != my_format_ext {
            Err(AUDCLNT_E_UNSUPPORTED_FORMAT)?
        }
        let buffer_size = hns_buffer_duration as f64 / sound::HNS_PER_SECOND as f64
            * MyAudioClient::SAMPLE_RATE as f64;
        let buffer_size = buffer_size.round() as usize;
        let event_mode = stream_flags & AUDCLNT_STREAMFLAGS_EVENTCALLBACK != 0;
        let event_handle = event_mode.then(OnceLock::new);
        let start = Mutex::default();
        let frame_counter = Arc::new(AtomicU64::new(0));
        let init = InitializedState {
            buffer_size,
            event_handle,
            frame_counter,
            start,
        };
        self.init.set(init).ok().unwrap();
        Ok(())
    }

    fn GetBufferSize(&self) -> windows_result::Result<u32> {
        log::trace!("MyAudioClient GetBufferSize");
        let init = self.init.get().ok_or(AUDCLNT_E_NOT_INITIALIZED)?;
        Ok(init.buffer_size as _)
    }

    fn GetStreamLatency(&self) -> windows_result::Result<i64> {
        log::trace!("MyAudioClient GetStreamLatency");
        if self.init.get().is_none() {
            Err(AUDCLNT_E_NOT_INITIALIZED)?
        }
        Ok(0)
    }

    fn GetCurrentPadding(&self) -> windows_result::Result<u32> {
        // log::trace!("MyAudioClient GetCurrentPadding");
        let init = self.init.get().ok_or(AUDCLNT_E_NOT_INITIALIZED)?;
        let start_guard = init.start.lock();
        let count = init.frame_counter.load(Ordering::Acquire);
        let pad = if let Some(start) = start_guard.as_ref() {
            let (pc, f) = timing::perf();
            let dt = pc - start.time;
            let dur_fr = dt as f64 * MyAudioClient::SAMPLE_RATE as f64 / f as f64;
            count as i64 - dur_fr.round() as i64
        } else {
            init.buffer_size as _
        };
        Ok(pad.clamp(0, init.buffer_size as _) as _)
    }

    fn IsFormatSupported(
        &self,
        share_mode: AUDCLNT_SHAREMODE,
        format: *const WAVEFORMATEX,
        out_closest_match: *mut *mut WAVEFORMATEX,
    ) -> HRESULT {
        log::trace!("MyAudioClient IsFormatSupported");
        if share_mode != AUDCLNT_SHAREMODE_SHARED {
            return AUDCLNT_E_UNSUPPORTED_FORMAT;
        }
        if format.is_null() || out_closest_match.is_null() {
            return E_POINTER;
        }
        unsafe {
            out_closest_match.write(std::ptr::null_mut());
        }
        let format_ex = unsafe { *format };
        if format_ex.wFormatTag != WAVE_FORMAT_EXTENSIBLE as u16 || format_ex.cbSize < 22 {
            return AUDCLNT_E_UNSUPPORTED_FORMAT;
        }
        let format_ext: WAVEFORMATEXTENSIBLE = unsafe { *format.cast() };
        let t_format_ext: [u8; size_of::<WAVEFORMATEXTENSIBLE>()] =
            unsafe { mem::transmute(format_ext) };
        let my_format_ext: [u8; size_of::<WAVEFORMATEXTENSIBLE>()] =
            unsafe { mem::transmute(self.wave_format) };
        if t_format_ext != my_format_ext {
            return AUDCLNT_E_UNSUPPORTED_FORMAT;
        }
        S_OK
    }

    fn GetMixFormat(&self) -> windows_result::Result<*mut WAVEFORMATEX> {
        log::trace!("MyAudioClient GetMixFormat");
        unsafe {
            let p = CoTaskMemAlloc(size_of::<WAVEFORMATEXTENSIBLE>()).cast();
            *p = self.wave_format;
            Ok(p.cast())
        }
    }

    fn GetDevicePeriod(
        &self,
        out_hns_default_device_period: *mut i64,
        out_hns_minimum_device_period: *mut i64,
    ) -> windows_result::Result<()> {
        log::trace!("MyAudioClient GetDevicePeriod");
        if out_hns_default_device_period.is_null() || out_hns_minimum_device_period.is_null() {
            Err(E_POINTER)?
        }
        unsafe {
            out_hns_default_device_period.write(100000);
            out_hns_minimum_device_period.write(30000);
        }
        Ok(())
    }

    fn Start(&self) -> windows_result::Result<()> {
        log::trace!("MyAudioClient Start");
        let init = self.init.get().ok_or(AUDCLNT_E_NOT_INITIALIZED)?;
        let mut start_guard = init.start.lock();
        if start_guard.is_some() {
            Err(AUDCLNT_E_NOT_STOPPED)?
        }
        if let Some(eh) = &init.event_handle {
            eh.get().ok_or(AUDCLNT_E_EVENTHANDLE_NOT_SET)?;
        }
        let (time, _) = timing::perf();
        let start = StartState { time };
        start_guard.replace(start);
        Ok(())
    }

    fn Stop(&self) -> windows_result::Result<()> {
        log::trace!("MyAudioClient Stop");
        let init = self.init.get().ok_or(AUDCLNT_E_NOT_INITIALIZED)?;
        log::debug!(
            "Buffer count: {}",
            init.frame_counter.load(Ordering::Relaxed)
        );
        init.start.lock().take().ok_or(S_FALSE)?;
        Ok(())
    }

    fn Reset(&self) -> windows_result::Result<()> {
        log::trace!("MyAudioClient Reset");
        self.init.get().ok_or(AUDCLNT_E_NOT_INITIALIZED)?;
        Ok(())
    }

    fn SetEventHandle(&self, event_handle: HANDLE) -> windows_result::Result<()> {
        log::trace!("MyAudioClient SetEventHandle");
        let init = self.init.get().ok_or(AUDCLNT_E_NOT_INITIALIZED)?;
        let e = Arc::new(event_handle.0 as _);
        EVENTS.lock().push(Arc::downgrade(&e));
        init.event_handle
            .as_ref()
            .ok_or(AUDCLNT_E_EVENTHANDLE_NOT_EXPECTED)?
            .set(e)
            .unwrap();
        Ok(())
    }

    fn GetService(
        &self,
        iid: *const GUID,
        out_v: *mut *mut std::ffi::c_void,
    ) -> windows_result::Result<()> {
        log::trace!("MyAudioClient GetService");
        let init = self.init.get().ok_or(AUDCLNT_E_NOT_INITIALIZED)?;
        if iid.is_null() || out_v.is_null() {
            Err(E_POINTER)?
        }
        let iid = unsafe { *iid };
        match iid {
            IAudioRenderClient::IID => {
                log::trace!("MyAudioClient GetService IAudioRenderClient");
                let a: IAudioRenderClient =
                    MyAudioRenderClient::new(init.buffer_size, init.frame_counter.clone()).into();
                unsafe {
                    *out_v = a.into_raw();
                }
                Ok(())
            }
            _ => Err(E_NOINTERFACE)?,
        }
    }
}
