use std::mem;

use windows_sys::{
    Win32::{
        Foundation::TRUE,
        System::{
            Console::AllocConsole,
            SystemServices::DLL_PROCESS_ATTACH,
        },
    },
    core::BOOL,
};

use crate::{
    envs,
    hooks,
};

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
extern "system" fn DllMain(
    _h_module: windows_sys::Win32::Foundation::HMODULE,
    reason: u32,
    _: *mut core::ffi::c_void,
) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        on_attach();
    }
    TRUE
}

fn on_attach() -> Option<()> {
    start_logger();
    alloc_console();
    hooks::init().inspect_err(|e| log::error!("{:?}", e)).ok()?;
    Some(())
}

fn alloc_console() {
    if *envs::ALLOC_CONSOLE {
        log::info!("Allocating Console Enabled");
        unsafe {
            AllocConsole();
        }
    }
}

fn start_logger() -> Option<()> {
    use flexi_logger::{
        FileSpec,
        Logger,
    };
    let mut logger = Logger::try_with_env().ok()?;
    if let Some(log_dir) = envs::LOG_DIR.as_ref() {
        logger = logger.log_to_file(FileSpec::default().directory(log_dir));
    }
    let handle = logger.start().ok()?;
    mem::forget(handle);
    Some(())
}

#[unsafe(no_mangle)]
static __MAGIC__: u64 = 1145141919810;
