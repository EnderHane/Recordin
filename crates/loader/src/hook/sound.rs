use std::ops::ControlFlow;

use crate::env;

mod wasapi;

const HNS_PER_SECOND: i64 = 10_000_000;

pub(super) fn com_hook(
    cls_id: *const windows_sys::core::GUID,
    outer: *mut core::ffi::c_void,
    cls_context: windows_sys::Win32::System::Com::CLSCTX,
    iid: *const windows_sys::core::GUID,
    out_v: *mut *mut core::ffi::c_void,
) -> ControlFlow<windows_sys::core::HRESULT> {
    #[allow(clippy::single_match)]
    match env::SOUND_SYSTEM.as_deref() {
        Some("wasapi") => {
            wasapi::com_hook(cls_id, outer, cls_context, iid, out_v)?;
        }
        _ => {}
    }
    ControlFlow::Continue(())
}

pub(super) fn lib_load_hook(_filename: &str, _h_module: usize) -> ControlFlow<anyhow::Result<()>> {
    ControlFlow::Continue(())
}

pub(super) fn init_early_loaded() -> Option<anyhow::Result<usize>> {
    None?
}
