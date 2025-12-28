use std::{
    ffi::CStr,
    ptr::NonNull,
};

use widestring::U16CStr;
use windows_sys::Win32::{
    Foundation::FALSE,
    System::Threading::{
        CREATE_SUSPENDED,
        CreateProcessA,
        CreateProcessW,
        PROCESS_INFORMATION,
        ResumeThread,
    },
};

use crate::{
    envs::TARGET_REGEX,
    inject,
};

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn CreateProcessW(
    p_app_name: windows_sys::core::PCWSTR,
    p_cmdline: windows_sys::core::PWSTR,
    p_process_attr: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    p_thread_attr: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    inherits_handles: windows_sys::core::BOOL,
    create_flags: windows_sys::Win32::System::Threading::PROCESS_CREATION_FLAGS,
    p_env: *const core::ffi::c_void,
    p_current_dir: windows_sys::core::PCWSTR,
    p_startup_info: *const windows_sys::Win32::System::Threading::STARTUPINFOW,
    out_process_info: *mut PROCESS_INFORMATION,
) -> windows_sys::core::BOOL {
    log::trace!("CreateProcessW");
    unsafe {
        let res = orig_CreateProcessW(
            p_app_name,
            p_cmdline,
            p_process_attr,
            p_thread_attr,
            inherits_handles,
            create_flags | CREATE_SUSPENDED,
            p_env,
            p_current_dir,
            p_startup_info,
            out_process_info,
        );
        if res != FALSE {
            let pi = *out_process_info;
            let app_name = NonNull::new(p_app_name.cast_mut())
                .map(|n| U16CStr::from_ptr_str(n.as_ptr()).to_string_lossy());
            let cmdline = NonNull::new(p_cmdline)
                .map(|n| U16CStr::from_ptr_str(n.as_ptr()).to_string_lossy());
            on_create(app_name.as_deref(), cmdline.as_deref(), create_flags, pi);
        }
        res
    }
}

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn CreateProcessA(
    p_app_name: windows_sys::core::PCSTR,
    p_cmdline: windows_sys::core::PSTR,
    p_process_attr: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    p_thread_attr: *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    inherits_handles: windows_sys::core::BOOL,
    create_flags: windows_sys::Win32::System::Threading::PROCESS_CREATION_FLAGS,
    p_env: *const core::ffi::c_void,
    p_current_dir: windows_sys::core::PCSTR,
    p_startup_info: *const windows_sys::Win32::System::Threading::STARTUPINFOA,
    out_process_info: *mut PROCESS_INFORMATION,
) -> windows_sys::core::BOOL {
    log::trace!("CreateProcessA");
    unsafe {
        let res = orig_CreateProcessA(
            p_app_name,
            p_cmdline,
            p_process_attr,
            p_thread_attr,
            inherits_handles,
            create_flags | CREATE_SUSPENDED,
            p_env,
            p_current_dir,
            p_startup_info,
            out_process_info,
        );
        if res != FALSE {
            let pi = *out_process_info;
            let app_name = NonNull::new(p_app_name.cast_mut())
                .map(|n| CStr::from_ptr(n.as_ptr().cast()).to_string_lossy());
            let cmdline = NonNull::new(p_cmdline)
                .map(|n| CStr::from_ptr(n.as_ptr().cast()).to_string_lossy());
            on_create(app_name.as_deref(), cmdline.as_deref(), create_flags, pi);
        }
        res
    }
}

pub(super) fn init() -> anyhow::Result<()> {
    unsafe {
        init_CreateProcessW(CreateProcessW)?.enable()?;
        init_CreateProcessA(CreateProcessA)?.enable()?;
    }
    Ok(())
}

fn on_create(
    app_name: Option<&str>,
    cmdline: Option<&str>,
    create_flags: u32,
    process_info: PROCESS_INFORMATION,
) -> Option<()> {
    let re = TARGET_REGEX.as_ref()?;
    let exe = if let Some(app_name) = app_name {
        app_name.to_owned()
    } else {
        cmdline.map(winsplit::split)?.first()?.clone()
    };
    if re.is_match(&exe) {
        inject::inject_self(process_info.hProcess);
    }
    if create_flags & CREATE_SUSPENDED == 0 {
        unsafe {
            ResumeThread(process_info.hThread);
        }
    }
    Some(())
}
