use std::{
    ops::ControlFlow,
    sync::{
        LazyLock,
        Weak,
    },
};

use kanal::Sender;
use parking_lot::Mutex;
use windows_sys::Win32::{
    Foundation::HANDLE,
    System::Threading::SetEvent,
};

use crate::envs;

mod audio_codec;
mod wasapi;

const HNS_PER_SECOND: i64 = 10_000_000;

static EVENTS: LazyLock<Mutex<Vec<Weak<usize>>>> = LazyLock::new(Mutex::default);

pub(super) static LISTENER: LazyLock<Sender<()>> = LazyLock::new(|| {
    let (tx, rx) = kanal::bounded(1);
    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            let mut guard = EVENTS.lock();
            for i in guard.len()..0 {
                let weak = &guard[i];
                if let Some(r) = weak.upgrade() {
                    let handle = *r as HANDLE;
                    unsafe {
                        SetEvent(handle);
                    }
                } else {
                    guard.swap_remove(i);
                }
            }
        }
    });
    tx
});

pub(super) fn com_hook(
    cls_id: *const windows_sys::core::GUID,
    outer: *mut core::ffi::c_void,
    cls_context: windows_sys::Win32::System::Com::CLSCTX,
    iid: *const windows_sys::core::GUID,
    out_v: *mut *mut core::ffi::c_void,
) -> ControlFlow<windows_sys::core::HRESULT> {
    if envs::AUDIO_OUTPUT.is_some() {
        #[allow(clippy::single_match)]
        match envs::SOUND_SYSTEM.as_deref() {
            Some("wasapi") => {
                wasapi::wasapi_hook(cls_id, outer, cls_context, iid, out_v)?;
            }
            _ => {}
        }
    }
    ControlFlow::Continue(())
}
