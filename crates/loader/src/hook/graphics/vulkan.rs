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
    if let Some(name) = path.file_stem()
        && name == "vulkan-1"
    {
        log::trace!("LoadLibrary vulkan-1.dll: {}", filename);
        unsafe {
            let lib_vulkan = Library::from_raw(module as _);
            let w = init_entry(&lib_vulkan);
            lib_vulkan.into_raw();
            ControlFlow::Break(w)?
        }
    }
    ControlFlow::Continue(())
}

pub(super) fn init_early_loaded() -> anyhow::Result<usize> {
    let lib_vulkan = Library::open_already_loaded("vulkan-1")?;
    log::trace!("LdrLoadDll vulkan-1.dll");
    init_entry(&lib_vulkan).ok();
    Ok(lib_vulkan.into_raw() as usize)
}

fn init_entry(lib_vulkan: &Library) -> anyhow::Result<()> {
    let get_instance_proc_addr = unsafe { lib_vulkan.get("vkGetInstanceProcAddr") }?;
    let get_device_proc_addr = unsafe { lib_vulkan.get("vkGetDeviceProcAddr") }?;
    let st_c = StaticCommands {
        get_device_proc_addr: *get_device_proc_addr,
        get_instance_proc_addr: *get_instance_proc_addr,
    };
    let entry = unsafe { Entry::from_commands(&st_c) };
    #[allow(non_snake_case)]
    let pfn_vkCreateInstance = ENTRY.get_or_init(|| entry).commands().create_instance;
    unsafe {
        instance::init_vkCreateInstance(pfn_vkCreateInstance)?.enable()?;
    }
    Ok(())
}
