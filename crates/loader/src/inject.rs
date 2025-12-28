use std::{
    mem,
    ptr,
};

use windows_result::Error as WinError;
use windows_sys::{
    Win32::{
        Foundation::HANDLE,
        System::{
            Diagnostics::Debug::WriteProcessMemory,
            LibraryLoader::{
                GetModuleFileNameW,
                GetModuleHandleW,
                GetProcAddress,
            },
            Memory::{
                MEM_COMMIT,
                PAGE_READWRITE,
                VirtualAllocEx,
            },
            Threading::{
                CreateRemoteThread,
                INFINITE,
                WaitForSingleObject,
            },
        },
    },
    w,
};

pub fn inject_self(h_process: HANDLE) {
    let h_module = hmod::current();
    let mut buf = vec![0; crate::MAX_PATH_W as usize];
    let path_len = unsafe { GetModuleFileNameW(h_module, buf.as_mut_ptr(), buf.len() as u32) };
    let h_k32 = unsafe { GetModuleHandleW(w!("kernel32.dll")) };
    #[allow(non_snake_case)]
    let proc_LoadLibraryW =
        unsafe { GetProcAddress(h_k32, c"LoadLibraryW".to_bytes_with_nul().as_ptr()) };
    let result = unsafe {
        let target_mem = VirtualAllocEx(
            h_process,
            ptr::null(),
            (path_len as usize + 1) * size_of::<u16>(),
            MEM_COMMIT,
            PAGE_READWRITE,
        );
        WriteProcessMemory(
            h_process,
            target_mem,
            buf.as_ptr().cast(),
            (path_len as usize + 1) * size_of::<u16>(),
            ptr::null_mut(),
        );
        #[allow(clippy::missing_transmute_annotations)]
        CreateRemoteThread(
            h_process,
            ptr::null(),
            0,
            mem::transmute(proc_LoadLibraryW),
            target_mem,
            0,
            ptr::null_mut(),
        )
    };
    if !result.is_null() {
        unsafe {
            WaitForSingleObject(result, INFINITE);
        }
    } else {
        log::error!("unable to inject self: {}", WinError::from_thread());
    }
}
