use std::slice;

use vulkanalia::{
    vk,
    vk::{
        DeviceV1_0,
        HasBuilder,
    },
};

use crate::{
    envs,
    hooks::{
        graphics::vulkan::{
            devices::{
                DEVICES,
                QUEUES,
            },
            swapchains::SWAPCHAINS,
        },
        times,
    },
};

#[allow(dead_code, non_snake_case)]
pub(super) unsafe extern "system" fn my_vkQueuePresentKHR(
    queue: vk::Queue,
    present_info: *const vk::PresentInfoKHR,
) -> vk::Result {
    #[cfg(debug_assertions)]
    {
        static CALL_TIME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let t = CALL_TIME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if t == 1200 {
            log::trace!("vkQueuePresentKHR x 1200");
        }
    }
    let d = QUEUES.get(&queue).unwrap();
    let dev_st = DEVICES.get(&d).unwrap();
    let info = unsafe { *present_info };
    let present_count = info.swapchain_count as usize;
    unsafe {
        let swapchains = slice::from_raw_parts(info.swapchains, present_count);
        let image_indices = slice::from_raw_parts(info.image_indices, present_count);
        let original_semaphores =
            slice::from_raw_parts(info.wait_semaphores, info.wait_semaphore_count as _);
        let mut new_semaphores = vec![];
        for i in 0..present_count {
            let chain = swapchains[i];
            let image_i = image_indices[i];
            let mut chain_st = SWAPCHAINS.get_mut(&chain).unwrap();
            new_semaphores.push(chain_st.copy_semaphore);
            let dev = &dev_st;
            let swap_image = chain_st.swap_images[image_i as usize];
            let cmd_buf = chain_st.command_buffer[0];
            let cmd_buf_begin_info = vk::CommandBufferBeginInfo::builder();
            dev.begin_command_buffer(cmd_buf, &cmd_buf_begin_info)
                .expect("Failed to begin command buffer");
            let barrier = vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::MEMORY_READ)
                .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
                .old_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(swap_image)
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1),
                );
            dev.cmd_pipeline_barrier(
                cmd_buf,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[] as &[vk::MemoryBarrier],
                &[] as &[vk::BufferMemoryBarrier],
                &[barrier],
            );
            let barrier = barrier
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .image(chain_st.dst_image);
            dev.cmd_pipeline_barrier(
                cmd_buf,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[] as &[vk::MemoryBarrier],
                &[] as &[vk::BufferMemoryBarrier],
                &[barrier],
            );
            let region = vk::ImageCopy::builder()
                .src_subresource(
                    vk::ImageSubresourceLayers::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .layer_count(1),
                )
                .dst_subresource(
                    vk::ImageSubresourceLayers::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .layer_count(1),
                )
                .extent(
                    vk::Extent3D::builder()
                        .width(chain_st.width)
                        .height(chain_st.height)
                        .depth(1),
                );
            dev.cmd_copy_image(
                cmd_buf,
                swap_image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                chain_st.dst_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
            let barrier = barrier
                .src_access_mask(vk::AccessFlags::TRANSFER_READ)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ)
                .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .image(swap_image);
            dev.cmd_pipeline_barrier(
                cmd_buf,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[] as &[vk::MemoryBarrier],
                &[] as &[vk::BufferMemoryBarrier],
                &[barrier],
            );
            let barrier = barrier
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::empty())
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::GENERAL)
                .image(chain_st.dst_image);
            dev.cmd_pipeline_barrier(
                cmd_buf,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[] as &[vk::MemoryBarrier],
                &[] as &[vk::BufferMemoryBarrier],
                &[barrier],
            );
            dev.end_command_buffer(cmd_buf)
                .expect("Failed to end command buffer");
            let cbs = [cmd_buf];
            let semas = [chain_st.copy_semaphore];
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(&cbs)
                .wait_semaphores(original_semaphores)
                .signal_semaphores(&semas);
            dev.queue_submit(dev_st.transfer_queue, &[submit_info], chain_st.fence)
                .expect("Failed to submit to queue");
            dev.wait_for_fences(&[chain_st.fence], true, u64::MAX)
                .expect("Failed to wait for fence");
            if envs::should_emit_video() {
                chain_st.post_copy();
            }
            dev.reset_fences(&[chain_st.fence])
                .expect("Failed to reset fence");
        }
        let mut new_present_info = info;
        new_present_info.wait_semaphore_count = new_semaphores.len() as _;
        new_present_info.wait_semaphores = new_semaphores.as_ptr();
        let res = dev_st.vkQueuePresentKHR()(queue, &new_present_info);
        times::incr_tick();
        res
    }
}
