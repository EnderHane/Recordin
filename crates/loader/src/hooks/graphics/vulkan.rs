mod devices;
mod instances;
mod preseting;
mod swapchains;

use std::sync::OnceLock;

use vulkanalia::{
    Entry,
    loader::{
        LIBRARY,
        LibloadingLoader,
    },
    vk::EntryV1_0,
};
static ENTRY: OnceLock<Entry> = OnceLock::new();

pub(super) fn init() -> anyhow::Result<()> {
    let loader = unsafe { LibloadingLoader::new(LIBRARY) }?;
    let entry = unsafe { Entry::new(loader) }.map_err(|e| anyhow::anyhow!("{e}"))?;
    #[allow(non_snake_case)]
    let pfn_vkCreateInstance = ENTRY.get_or_init(|| entry).commands().create_instance;
    unsafe {
        instances::init_vkCreateInstance(pfn_vkCreateInstance)?.enable()?;
    }
    Ok(())
}
