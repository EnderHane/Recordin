mod device;
mod instance;
mod present;
mod swap_chain;

use std::{
    ops::ControlFlow,
    path::Path,
    sync::OnceLock,
};

use libloading::os::windows::Library;
use vulkanalia::{
    Entry,
    vk::{
        EntryV1_0,
        StaticCommands,
    },
};

static ENTRY: OnceLock<Entry> = OnceLock::new();

pub(super) fn lib_load_hook(filename: &str, module: usize) -> ControlFlow<anyhow::Result<()>> {
    let path: &Path = filename.as_ref();
    let name = path.file_stem().unwrap();
    if name == "vulkan-1" {
        log::trace!("LoadLibrary vulkan-1.dll: {}", filename);
        unsafe {
            let lib = Library::from_raw(module as _);
            if let Some(r) = init(&lib) {
                lib.into_raw();
                ControlFlow::Break(r)?
            }
        }
    }
    ControlFlow::Continue(())
}

pub(super) fn init_early_loaded() -> Option<anyhow::Result<usize>> {
    let lib = Library::open_already_loaded("vulkan-1").ok()?;
    log::trace!("LdrLoadDll vulkan-1.dll");
    let r = init(&lib)?;
    Some(r.map(|_| lib.into_raw() as usize))
}

fn init(lib: &Library) -> Option<anyhow::Result<()>> {
    #[allow(non_snake_case)]
    unsafe {
        let pfn_vkGetInstanceProcAddr = *lib.get("vkGetInstanceProcAddr").ok()?;
        let pfn_vkGetDeviceProcAddr = *lib.get("vkGetDeviceProcAddr").ok()?;
        let st_c = StaticCommands {
            get_device_proc_addr: pfn_vkGetDeviceProcAddr,
            get_instance_proc_addr: pfn_vkGetInstanceProcAddr,
        };
        let entry = Entry::from_commands(&st_c);
        let pfn_vkCreateInstance = ENTRY.get_or_init(|| entry).commands().create_instance;
        let a = || {
            instance::init_vkCreateInstance(pfn_vkCreateInstance)?.enable()?;
            Ok(())
        };
        Some(a())
    }
}
