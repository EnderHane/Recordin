use std::{
    mem,
    ops::Deref,
    sync::LazyLock,
};

use dashmap::DashMap;
use retour::GenericDetour;
use vulkanalia::{
    Instance,
    vk,
    vk::InstanceV1_0,
};

use crate::hook::graphics::vulkan::{
    ENTRY,
    device,
};

#[recordin_macro::static_hook]
#[allow(dead_code)]
pub(super) unsafe extern "system" fn vkCreateInstance(
    create_info: *const vk::InstanceCreateInfo,
    allocator: *const vk::AllocationCallbacks,
    instance: *mut vk::Instance,
) -> vk::Result {
    log::trace!("vkCreateInstance");
    let res = unsafe { orig_vkCreateInstance(create_info, allocator, instance) };
    if res == vk::Result::SUCCESS {
        unsafe {
            let i = *instance;
            let info = *create_info;
            let fancy_instance = Instance::from_created(ENTRY.wait(), &info, i).unwrap();
            let phy_devs = fancy_instance.enumerate_physical_devices().unwrap();
            let Ok(instance_hook) = InstanceState::new(fancy_instance) else {
                return res;
            };
            for p in phy_devs {
                PHYSICAL_DEVICES.insert(p, i);
            }
            INSTANCES.insert(i, instance_hook);
        }
    }
    res
}

pub(super) static INSTANCES: LazyLock<DashMap<vk::Instance, InstanceState>> =
    LazyLock::new(DashMap::new);
pub(super) static PHYSICAL_DEVICES: LazyLock<DashMap<vk::PhysicalDevice, vk::Instance>> =
    LazyLock::new(DashMap::new);

#[derive(Debug)]
#[allow(non_snake_case)]
pub(super) struct InstanceState {
    #[allow(dead_code)]
    instance: Instance,
    hook_vkDestroyInstance: GenericDetour<vk::PFN_vkDestroyInstance>,
    hook_vkCreateDevice: GenericDetour<vk::PFN_vkCreateDevice>,
}

impl Deref for InstanceState {
    type Target = Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

impl InstanceState {
    #[allow(non_snake_case)]
    fn new(instance: Instance) -> Result<Self, retour::Error> {
        let pfn_vkDestroyInstance = instance.commands().destroy_instance;
        let pfn_vkCreateDevice = instance.commands().create_device;
        let hook_vkCreateDevice = unsafe {
            GenericDetour::new(pfn_vkCreateDevice, device::my_vkCreateDevice).and_then(|h| {
                h.enable()?;
                Ok(h)
            })?
        };
        let hook_vkDestroyInstance = unsafe {
            GenericDetour::new(pfn_vkDestroyInstance, my_vkDestroyInstance).and_then(|h| {
                h.enable()?;
                Ok(h)
            })?
        };
        Ok(Self {
            instance,
            hook_vkDestroyInstance,
            hook_vkCreateDevice,
        })
    }

    #[allow(non_snake_case)]
    unsafe fn vkDestroyInstance(&self) -> vk::PFN_vkDestroyInstance {
        unsafe { mem::transmute(self.hook_vkDestroyInstance.trampoline()) }
    }

    #[allow(non_snake_case)]
    pub(super) unsafe fn vkCreateDevice(&self) -> vk::PFN_vkCreateDevice {
        unsafe { mem::transmute(self.hook_vkCreateDevice.trampoline()) }
    }
}

#[allow(dead_code, non_snake_case)]
unsafe extern "system" fn my_vkDestroyInstance(
    instance: vk::Instance,
    allocator: *const vk::AllocationCallbacks,
) {
    log::trace!("vkDestroyInstance");
    let (_, inst_state) = INSTANCES.remove(&instance).unwrap();
    PHYSICAL_DEVICES.retain(|_, i| i != &instance);
    unsafe {
        inst_state.vkDestroyInstance()(instance, allocator);
    }
}
