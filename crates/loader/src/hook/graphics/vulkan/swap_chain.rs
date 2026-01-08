use std::sync::{
    LazyLock,
    atomic::{
        AtomicPtr,
        Ordering,
    },
};

use dashmap::DashMap;
use vulkanalia::{
    VkResult,
    vk,
    vk::{
        DeviceV1_0,
        HasBuilder,
        InstanceV1_0,
        KhrSwapchainExtensionDeviceCommands,
    },
};

use crate::{
    hook::{
        graphics,
        graphics::vulkan::{
            device::DEVICES,
            instance::{
                INSTANCES,
                PHYSICAL_DEVICES,
            },
        },
        timing,
    },
    output::{
        video_codec,
        video_codec::EncDuplex,
    },
};

#[allow(dead_code, non_snake_case)]
pub(super) unsafe extern "system" fn my_vkCreateSwapchainKHR(
    device: vk::Device,
    create_info: *const vk::SwapchainCreateInfoKHR,
    allocator: *const vk::AllocationCallbacks,
    swap_chain: *mut vk::SwapchainKHR,
) -> vk::Result {
    log::trace!("vkCreateSwapchainKHR");
    let dev_st = DEVICES.get(&device).expect("device not found");
    let res = unsafe { dev_st.vkCreateSwapchainKHR()(device, create_info, allocator, swap_chain) };
    if res != vk::Result::SUCCESS {
        return res;
    }
    unsafe {
        let info = *create_info;
        let chain = *swap_chain;
        log::debug!(
            "Create VkSwapchainKHR@{chain:?} on VkDevice@{device:?}, \
                format: {:?}, color space: {:?}, extent: {:?}",
            info.image_format,
            info.image_color_space,
            info.image_extent,
        );
        let chain_state = SwapChainState::new(device, info, chain).unwrap();
        SWAP_CHAINS.insert(chain, chain_state);
    }
    res
}

pub(super) static SWAP_CHAINS: LazyLock<DashMap<vk::SwapchainKHR, SwapChainState>> =
    LazyLock::new(DashMap::new);

#[allow(dead_code, non_snake_case)]
pub(super) unsafe extern "system" fn my_vkDestroySwapchainKHR(
    device: vk::Device,
    swap_chain: vk::SwapchainKHR,
    allocator: *const vk::AllocationCallbacks,
) {
    log::trace!("vkDestroySwapchainKHR");
    log::debug!("Destroy VkSwapchainKHR@{swap_chain:?} on VkDevice@{device:?}");
    let dev_st = DEVICES.get(&device).unwrap();
    let (_, chain_st) = SWAP_CHAINS.remove(&swap_chain).unwrap();
    let fr = chain_st.frame_count as f64;
    let (t, f) = timing::real();
    let dT = (t - chain_st.init_real_time) as f64;
    log::debug!("Average FPS: {}", fr / dT * f as f64);
    unsafe {
        dev_st.free_command_buffers(dev_st.command_pool, &chain_st.command_buffer);
        dev_st.destroy_semaphore(chain_st.copy_semaphore, None);
        dev_st.destroy_fence(chain_st.fence, None);
        dev_st.unmap_memory(chain_st.dst_memory);
        dev_st.free_memory(chain_st.dst_memory, None);
        dev_st.destroy_image(chain_st.dst_image, None);
        dev_st.vkDestroySwapchainKHR()(device, swap_chain, allocator);
    }
}

#[derive(Debug)]
pub(super) struct SwapChainState {
    pub(super) swap_images: Vec<vk::Image>,
    pub(super) copy_semaphore: vk::Semaphore,
    pub(super) fence: vk::Fence,
    pub(super) command_buffer: Vec<vk::CommandBuffer>,
    pub(super) dst_image: vk::Image,
    pub(super) dst_memory: vk::DeviceMemory,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) row_pitch: vk::DeviceSize,
    pub(super) mapped: AtomicPtr<core::ffi::c_void>,
    pub(super) encoder: Option<EncDuplex>,
    pub(super) init_real_time: i64,
    pub(super) frame_count: u64,
}

impl SwapChainState {
    fn new(
        device: vk::Device,
        info: vk::SwapchainCreateInfoKHR,
        chain: vk::SwapchainKHR,
    ) -> VkResult<Self> {
        unsafe {
            let dev_st = DEVICES.get(&device).unwrap();
            let command_buffer_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(dev_st.command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            let command_buffer = dev_st.allocate_command_buffers(&command_buffer_info)?;
            let semaphore_info = vk::SemaphoreCreateInfo::builder();
            let copy_semaphore = dev_st.create_semaphore(&semaphore_info, None)?;
            let fence_info = vk::FenceCreateInfo::builder();
            let fence = dev_st.create_fence(&fence_info, None)?;
            let swap_images = dev_st.get_swapchain_images_khr(chain)?;
            let width = info.image_extent.width;
            let height = info.image_extent.height;
            let image_info = vk::ImageCreateInfo::builder()
                .image_type(vk::ImageType::_2D)
                .format(vk::Format::B8G8R8A8_UNORM)
                .extent(vk::Extent3D::builder().width(width).height(height).depth(1))
                .array_layers(1)
                .mip_levels(1)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .samples(vk::SampleCountFlags::_1)
                .tiling(vk::ImageTiling::LINEAR)
                .usage(vk::ImageUsageFlags::TRANSFER_DST);
            let dst_image = dev_st.create_image(&image_info, None)?;
            let mem_req = dev_st.get_image_memory_requirements(dst_image);
            let phy_dev = dev_st.physical_device();
            let instance = PHYSICAL_DEVICES.get(&phy_dev).unwrap();
            let inst_st = INSTANCES.get(&instance).unwrap();
            let pd_mem_props = inst_st.get_physical_device_memory_properties(phy_dev);
            let type_bits = mem_req.memory_type_bits;
            let desired_prop = vk::MemoryPropertyFlags::HOST_VISIBLE
                | vk::MemoryPropertyFlags::HOST_COHERENT
                | vk::MemoryPropertyFlags::HOST_CACHED;
            let type_index = (0..pd_mem_props.memory_type_count)
                .filter(|&i| type_bits & (1u32 << i) != 0)
                .find(|&i| {
                    pd_mem_props.memory_types[i as usize]
                        .property_flags
                        .contains(desired_prop)
                })
                .unwrap_or(0);
            let mem_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(mem_req.size)
                .memory_type_index(type_index);
            let dst_memory = dev_st.allocate_memory(&mem_info, None)?;
            dev_st.bind_image_memory(dst_image, dst_memory, 0)?;
            let sub_res = vk::ImageSubresource::builder().aspect_mask(vk::ImageAspectFlags::COLOR);
            let sub_res_layout = dev_st.get_image_subresource_layout(dst_image, &sub_res);
            let row_pitch = sub_res_layout.row_pitch;
            let offset = sub_res_layout.offset;
            let mapped = dev_st.map_memory(
                dst_memory,
                offset,
                vk::WHOLE_SIZE,
                vk::MemoryMapFlags::empty(),
            )?;
            let mapped = AtomicPtr::new(mapped);
            let encoder = video_codec::create_encoder(width as _, height as _);
            Ok(Self {
                swap_images,
                copy_semaphore,
                fence,
                command_buffer,
                dst_image,
                dst_memory,
                width,
                height,
                row_pitch,
                mapped,
                encoder,
                init_real_time: timing::real().0,
                frame_count: 0,
            })
        }
    }
}

impl SwapChainState {
    pub(super) fn pre_copy(&mut self) -> Option<()> {
        self.frame_count += 1;
        Some(())
    }

    pub(super) fn post_copy(&mut self) -> Option<()> {
        unsafe {
            let mut packed_bgr = self.encoder.as_ref()?.1.recv().ok()?;
            packed_bgr.resize((self.width * self.height) as _, [0; _]);
            let packed_lines = packed_bgr.chunks_exact_mut(self.width as usize);
            let mapped_slices = graphics::slices_by_row_pitch(
                self.mapped.load(Ordering::Relaxed).cast(),
                (self.width * 4) as _,
                self.height as _,
                self.row_pitch as _,
            );
            let z = packed_lines.zip(mapped_slices);
            for (packed_line, mapped_slice) in z {
                let (raw_c, _) = mapped_slice.as_chunks();
                let zz = packed_line.iter_mut().zip(raw_c);
                for (packed, &[b, g, r, _]) in zz {
                    *packed = [b, g, r];
                }
            }
            self.encoder.as_ref()?.0.send(packed_bgr).ok()?;
            Some(())
        }
    }
}
