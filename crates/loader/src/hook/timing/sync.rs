use std::{
    ptr,
    slice,
    sync::{
        LazyLock,
        atomic::Ordering,
    },
};

use arrayvec::ArrayVec;
use parking_lot::RwLock;
use windows_sys::{
    Win32::{
        Foundation::{
            FALSE,
            HANDLE,
            TRUE,
            WAIT_EVENT,
            WAIT_OBJECT_0,
            WAIT_TIMEOUT,
        },
        System::Threading::{
            CreateEventW,
            INFINITE,
            ResetEvent,
            SetEvent,
        },
    },
    core::BOOL,
};

pub(super) static RESYNC_EVENT: LazyLock<usize> =
    LazyLock::new(|| unsafe { CreateEventW(ptr::null(), TRUE, TRUE, ptr::null()) } as _);
pub(super) static NEXT_FRAME_EVENT: LazyLock<usize> =
    LazyLock::new(|| unsafe { CreateEventW(ptr::null(), TRUE, TRUE, ptr::null()) } as _);
pub(super) static WAITING: RwLock<()> = RwLock::new(());

pub(super) fn tick() {
    unsafe {
        ResetEvent(*RESYNC_EVENT as _);
        SetEvent(*NEXT_FRAME_EVENT as _);
        {
            let _w = WAITING.write();
            ResetEvent(*NEXT_FRAME_EVENT as _);
            SetEvent(*RESYNC_EVENT as _);
        }
    }
}

#[recordin_macro::static_hook]
#[allow(dead_code)]
pub(super) unsafe extern "system" fn WaitForSingleObject(handle: HANDLE, ms: u32) -> WAIT_EVENT {
    // log::trace!("WaitForSingleObjects {ms}");
    unsafe {
        if !super::ENABLED.load(Ordering::Acquire) || ms > 0x7fffffff {
            orig_WaitForSingleObject(handle, ms)
        } else {
            orig_WaitForSingleObject(*RESYNC_EVENT as _, INFINITE);
            let new_handles = [handle, *NEXT_FRAME_EVENT as _];
            let res = {
                let _a = WAITING.read();
                orig_WaitForMultipleObjects(new_handles.len() as _, new_handles.as_ptr(), FALSE, ms)
            };
            if res == WAIT_OBJECT_0 + 1 {
                WAIT_TIMEOUT
            } else {
                res
            }
        }
    }
}

#[recordin_macro::static_hook]
#[allow(dead_code)]
pub(super) unsafe extern "system" fn WaitForMultipleObjects(
    count: u32,
    p_handles: *const HANDLE,
    wait_all: BOOL,
    ms: u32,
) -> WAIT_EVENT {
    // log::trace!("WaitForMultipleObjects {count} {ms}");
    unsafe {
        if !super::ENABLED.load(Ordering::Acquire) || ms > 0x7fffffff {
            orig_WaitForMultipleObjects(count, p_handles, wait_all, ms)
        } else if wait_all == TRUE {
            orig_WaitForMultipleObjects(count, p_handles, TRUE, 0)
        } else {
            orig_WaitForSingleObject(*RESYNC_EVENT as _, INFINITE);
            let handles = slice::from_raw_parts(p_handles, count as usize);
            let mut new_handles = ArrayVec::<HANDLE, 16>::new();
            new_handles.extend(handles.iter().copied());
            new_handles.push(*NEXT_FRAME_EVENT as _);
            let res = {
                let _a = WAITING.read();
                orig_WaitForMultipleObjects(new_handles.len() as _, new_handles.as_ptr(), FALSE, ms)
            };
            if res == WAIT_OBJECT_0 + count {
                WAIT_TIMEOUT
            } else {
                res
            }
        }
    }
}
