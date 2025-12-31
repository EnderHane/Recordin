use std::{
    ffi::CStr,
    ops::ControlFlow,
    sync::LazyLock,
};

use dashmap::DashSet;
use widestring::U16CStr;
use windows_sys::Win32::{
    Foundation::{
        FreeLibrary,
        HMODULE,
        TRUE,
    },
    System::LibraryLoader::{
        LoadLibraryA,
        LoadLibraryExA,
        LoadLibraryExW,
        LoadLibraryW,
    },
};

use crate::hook::graphics;

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn LoadLibraryW(lib_filename: windows_sys::core::PCWSTR) -> HMODULE {
    // log::trace!("LoadLibraryW");
    let h = unsafe { orig_LoadLibraryW(lib_filename) };
    let filename = unsafe { U16CStr::from_ptr_str(lib_filename) }.to_string_lossy();
    on_load_library(&filename, h);
    h
}

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn LoadLibraryA(lib_filename: windows_sys::core::PCSTR) -> HMODULE {
    // log::trace!("LoadLibraryA");
    let h = unsafe { orig_LoadLibraryA(lib_filename) };
    let filename = unsafe { CStr::from_ptr(lib_filename.cast()) }.to_string_lossy();
    on_load_library(&filename, h);
    h
}

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn LoadLibraryExA(
    lib_filename: windows_sys::core::PCSTR,
    file: HMODULE,
    flags: u32,
) -> HMODULE {
    // log::trace!("LoadLibraryExA");
    let h = unsafe { orig_LoadLibraryExA(lib_filename, file, flags) };
    let filename = unsafe { CStr::from_ptr(lib_filename.cast()) }.to_string_lossy();
    on_load_library(&filename, h);
    h
}

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn LoadLibraryExW(
    lib_filename: windows_sys::core::PCWSTR,
    file: HMODULE,
    flags: u32,
) -> HMODULE {
    // log::trace!("LoadLibraryExW");
    let h = unsafe { orig_LoadLibraryExW(lib_filename, file, flags) };
    let filename = unsafe { U16CStr::from_ptr_str(lib_filename) }.to_string_lossy();
    on_load_library(&filename, h);
    h
}

static HOOKED: LazyLock<DashSet<usize>> = LazyLock::new(DashSet::new);

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn FreeLibrary(module: HMODULE) -> windows_sys::core::BOOL {
    // log::trace!("FreeLibrary");
    unsafe {
        if HOOKED.contains(&module.addr()) {
            TRUE
        } else {
            orig_FreeLibrary(module)
        }
    }
}

fn on_load_library(filename: &str, module: HMODULE) {
    if module.is_null() {
        return;
    }
    let h_module = module as usize;
    if HOOKED.contains(&h_module) {
        return;
    }
    let chain = || {
        graphics::lib_load_hook(filename, h_module)?;
        ControlFlow::Continue(())
    };
    if let ControlFlow::Break(Ok(_)) = chain() {
        HOOKED.insert(h_module);
    }
}

pub(super) fn init() -> anyhow::Result<()> {
    unsafe {
        init_LoadLibraryW(LoadLibraryW)?.enable()?;
        init_LoadLibraryA(LoadLibraryA)?.enable()?;
        init_LoadLibraryExA(LoadLibraryExA)?.enable()?;
        init_LoadLibraryExW(LoadLibraryExW)?.enable()?;
        init_FreeLibrary(FreeLibrary)?.enable()?;
    }
    init_early_loaded().ok();
    Ok(())
}

fn init_early_loaded() -> anyhow::Result<()> {
    if let Some(a) = graphics::init_early_loaded() {
        HOOKED.insert(a?);
    }
    Ok(())
}
