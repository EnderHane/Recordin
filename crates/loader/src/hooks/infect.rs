use windows_sys::Win32::{
    Foundation::FALSE,
    System::Threading::{
        CREATE_SUSPENDED,
        CreateProcessA,
        CreateProcessW,
        ResumeThread,
    },
};

use crate::{
    envs::{
        should_inject_a,
        should_inject_w,
    },
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
    out_process_info: *mut windows_sys::Win32::System::Threading::PROCESS_INFORMATION,
) -> windows_sys::core::BOOL {
    log::trace!("CreateProcessW");
    let should_inject = unsafe { should_inject_w(p_app_name, p_cmdline) };
    let mut new_flags = create_flags;
    if should_inject {
        new_flags |= CREATE_SUSPENDED;
    }
    let res = unsafe {
        orig_CreateProcessW(
            p_app_name,
            p_cmdline,
            p_process_attr,
            p_thread_attr,
            inherits_handles,
            new_flags,
            p_env,
            p_current_dir,
            p_startup_info,
            out_process_info,
        )
    };
    if should_inject && res != FALSE {
        let pi = unsafe { *out_process_info };
        inject::inject_self(pi.hProcess);
        if create_flags & CREATE_SUSPENDED == 0 {
            unsafe {
                ResumeThread(pi.hThread);
            }
        }
    }
    res
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
    out_process_info: *mut windows_sys::Win32::System::Threading::PROCESS_INFORMATION,
) -> windows_sys::core::BOOL {
    log::trace!("CreateProcessA");
    let should_inject = unsafe { should_inject_a(p_app_name, p_cmdline) };
    let mut new_flags = create_flags;
    if should_inject {
        new_flags |= CREATE_SUSPENDED;
    }
    let res = unsafe {
        orig_CreateProcessA(
            p_app_name,
            p_cmdline,
            p_process_attr,
            p_thread_attr,
            inherits_handles,
            new_flags,
            p_env,
            p_current_dir,
            p_startup_info,
            out_process_info,
        )
    };
    if should_inject && res != FALSE {
        let pi = unsafe { *out_process_info };
        inject::inject_self(pi.hProcess);
        if create_flags & CREATE_SUSPENDED == 0 {
            unsafe {
                ResumeThread(pi.hThread);
            }
        }
    }
    res
}

pub(super) fn init() -> anyhow::Result<()> {
    unsafe {
        init_CreateProcessW(CreateProcessW)?.enable()?;
        init_CreateProcessA(CreateProcessA)?.enable()?;
    }
    Ok(())
}
