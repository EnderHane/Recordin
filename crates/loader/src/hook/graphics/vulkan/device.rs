use std::{
    mem,
    ops::Deref,
    sync::LazyLock,
};

use dashmap::DashMap;
use retour::GenericDetour;
use vulkanalia::{
    Device,
    vk,
    vk::{
        DeviceV1_0,
        HasBuilder,
        InstanceV1_0,
    },
};

use crate::hook::{
    graphics::vulkan::{
        ENTRY,
        instance::{
            INSTANCES,
            PHYSICAL_DEVICES,
        },
        present,
        swap_chain,
    },
    timing,
};

pub(super) static DEVICES: LazyLock<DashMap<vk::Device, DeviceState>> = LazyLock::new(DashMap::new);
pub(super) static QUEUES: LazyLock<DashMap<vk::Queue, vk::Device>> = LazyLock::new(DashMap::new);

#[allow(dead_code, non_snake_case)]
pub(super) unsafe extern "system" fn my_vkCreateDevice(
    phy_dev @ physical_device: vk::PhysicalDevice,
    create_info: *const vk::DeviceCreateInfo,
    allocator: *const vk::AllocationCallbacks,
    p_device: *mut vk::Device,
) -> vk::Result {
    log::trace!("vkCreateDevice");
    let instance = PHYSICAL_DEVICES
        .get(&physical_device)
        .expect("physical device not found");
    let inst_state = INSTANCES.get(instance.value()).expect("instance not found");
    let res = unsafe { inst_state.vkCreateDevice()(phy_dev, create_info, allocator, p_device) };
    if res != vk::Result::SUCCESS {
        return res;
    }
    unsafe {
        let d = *p_device;
        let info = *create_info;
        let fancy_device = Device::from_created(ENTRY.wait(), phy_dev, &info, d).unwrap();
        let queue_properties =
            inst_state.get_physical_device_queue_family_properties(physical_device);
        let i_queue_family = queue_properties
            .iter()
            .position(|p| p.queue_flags.contains(vk::QueueFlags::TRANSFER))
            .unwrap_or(0);
        for (i, qp) in queue_properties.iter().enumerate() {
            for j in 0..qp.queue_count {
                let q = fancy_device.get_device_queue(i as _, j);
                QUEUES.insert(q, d);
            }
        }
        let device_state = DeviceState::new(fancy_device, i_queue_family as u32).unwrap();
        DEVICES.insert(d, device_state);
    }
    res
}

#[derive(Debug)]
#[allow(non_snake_case)]
pub(super) struct DeviceState {
    #[allow(dead_code)]
    device: Device,
    pub(super) transfer_queue: vk::Queue,
    pub(super) command_pool: vk::CommandPool,
    hook_vkDestroyDevice: GenericDetour<vk::PFN_vkDestroyDevice>,
    hook_vkQueuePresentKHR: GenericDetour<vk::PFN_vkQueuePresentKHR>,
    hook_vkCreateSwapchainKHR: GenericDetour<vk::PFN_vkCreateSwapchainKHR>,
    hook_vkDestroySwapchainKHR: GenericDetour<vk::PFN_vkDestroySwapchainKHR>,
}

impl Deref for DeviceState {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl DeviceState {
    #[allow(non_snake_case)]
    fn new(device: Device, queue_family_index: u32) -> anyhow::Result<Self> {
        let pfn_vkDestroyDevice = device.commands().destroy_device;
        let pfn_vkQueuePresentKHR = device.commands().queue_present_khr;
        let pfn_vkCreateSwapchainKHR = device.commands().create_swapchain_khr;
        let pfn_vkDestroySwapchainKHR = device.commands().destroy_swapchain_khr;
        let hook_vkDestroyDevice = unsafe {
            GenericDetour::new(pfn_vkDestroyDevice, my_vkDestroyDevice).and_then(|h| {
                h.enable()?;
                Ok(h)
            })?
        };
        let hook_vkQueuePresentKHR = unsafe {
            GenericDetour::new(pfn_vkQueuePresentKHR, present::my_vkQueuePresentKHR).and_then(
                |h| {
                    h.enable()?;
                    Ok(h)
                },
            )?
        };
        let hook_vkCreateSwapchainKHR = unsafe {
            GenericDetour::new(
                pfn_vkCreateSwapchainKHR,
                swap_chain::my_vkCreateSwapchainKHR,
            )
            .and_then(|h| {
                h.enable()?;
                Ok(h)
            })?
        };
        let hook_vkDestroySwapchainKHR = unsafe {
            GenericDetour::new(
                pfn_vkDestroySwapchainKHR,
                swap_chain::my_vkDestroySwapchainKHR,
            )
            .and_then(|h| {
                h.enable()?;
                Ok(h)
            })?
        };
        let transfer_queue = unsafe { device.get_device_queue(queue_family_index, 0) };
        let command_pool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe { device.create_command_pool(&command_pool_info, None) }?;
        Ok(Self {
            device,
            transfer_queue,
            command_pool,
            hook_vkDestroyDevice,
            hook_vkQueuePresentKHR,
            hook_vkCreateSwapchainKHR,
            hook_vkDestroySwapchainKHR,
        })
    }

    #[allow(non_snake_case)]
    unsafe fn vkDestroyDevice(&self) -> vk::PFN_vkDestroyDevice {
        unsafe { mem::transmute(self.hook_vkDestroyDevice.trampoline()) }
    }

    #[allow(non_snake_case)]
    pub(super) unsafe fn vkQueuePresentKHR(&self) -> vk::PFN_vkQueuePresentKHR {
        unsafe { mem::transmute(self.hook_vkQueuePresentKHR.trampoline()) }
    }

    #[allow(non_snake_case)]
    pub(super) unsafe fn vkCreateSwapchainKHR(&self) -> vk::PFN_vkCreateSwapchainKHR {
        unsafe { mem::transmute(self.hook_vkCreateSwapchainKHR.trampoline()) }
    }

    #[allow(non_snake_case)]
    pub(super) unsafe fn vkDestroySwapchainKHR(&self) -> vk::PFN_vkDestroySwapchainKHR {
        unsafe { mem::transmute(self.hook_vkDestroySwapchainKHR.trampoline()) }
    }
}

#[allow(dead_code, non_snake_case)]
unsafe extern "system" fn my_vkDestroyDevice(
    device: vk::Device,
    allocator: *const vk::AllocationCallbacks,
) {
    log::trace!("vkDestroyDevice");
    let (_, dev_state) = DEVICES.remove(&device).expect("device not found");
    unsafe {
        dev_state.vkDestroyDevice()(device, allocator);
    }
    if DEVICES.is_empty() {
        timing::pause();
    }
}
