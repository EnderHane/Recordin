use std::sync::atomic::Ordering;

use windows_sys::Win32::System::Threading::INFINITE;

use crate::hook::timing::{
    sync,
    sync::{
        NEXT_FRAME_EVENT,
        RESYNC_EVENT,
        orig_WaitForSingleObject,
    },
};

#[recordin_macro::static_hook]
#[allow(dead_code)]
pub(super) unsafe extern "system" fn Sleep(ms: u32) {
    // log::trace!("Sleep");
    unsafe {
        if !super::ENABLED.load(Ordering::Acquire) || ms == 0 {
            orig_Sleep(ms)
        } else {
            orig_WaitForSingleObject(*RESYNC_EVENT as _, INFINITE);
            {
                let _a = sync::WAITING.read();
                orig_WaitForSingleObject(*NEXT_FRAME_EVENT as _, ms);
            }
        }
    }
}
