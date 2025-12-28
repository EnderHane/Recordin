use std::{
    iter::FusedIterator,
    marker::PhantomData,
    ops::ControlFlow,
    slice,
};

use crate::envs;

mod video_codec;
mod vulkan;

pub(super) fn lib_load_hook(filename: &str, h_module: usize) -> ControlFlow<anyhow::Result<()>> {
    if envs::FPS.is_some() {
        #[allow(clippy::single_match)]
        match envs::GRAPHICS_SYSTEM.as_deref() {
            Some("vulkan") => {
                vulkan::lib_load_hook(filename, h_module)?;
            }
            _ => {}
        }
    }
    ControlFlow::Continue(())
}

pub(super) fn init_early_loaded() -> Option<anyhow::Result<usize>> {
    envs::FPS.as_ref()?;
    match envs::GRAPHICS_SYSTEM.as_deref()? {
        "vulkan" => Some(vulkan::init_early_loaded()),
        "d3d12" => unimplemented!(),
        _ => None,
    }
}

#[derive(Debug)]
struct SlicesByRowPitch<'m> {
    data: *const u8,
    width: usize,
    row_pitch: usize,
    forward: usize,
    backward: usize,
    _marker: PhantomData<&'m ()>,
}

impl<'m> Iterator for SlicesByRowPitch<'m> {
    type Item = &'m [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.forward == self.backward {
            None?;
        }
        let p = self.data.wrapping_add(self.forward * self.row_pitch);
        self.forward += 1;
        Some(unsafe { slice::from_raw_parts(p, self.width) })
    }
}

impl<'m> DoubleEndedIterator for SlicesByRowPitch<'m> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.backward == self.forward {
            None?;
        }
        let p = self.data.wrapping_add(self.backward * self.row_pitch);
        self.forward -= 1;
        Some(unsafe { slice::from_raw_parts(p, self.width) })
    }
}

impl FusedIterator for SlicesByRowPitch<'_> {}

unsafe fn slices_by_row_pitch<'m>(
    data: *const u8,
    width: usize,
    height: usize,
    row_pitch: usize,
) -> SlicesByRowPitch<'m> {
    SlicesByRowPitch {
        data,
        width,
        row_pitch,
        forward: 0,
        backward: height - 1,
        _marker: PhantomData,
    }
}
