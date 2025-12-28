use windows::Win32::{
    Foundation::E_INVALIDARG,
    Media::Audio::{
        IMMDevice,
        IMMDeviceCollection,
        IMMDeviceCollection_Impl,
    },
};
use windows_core::implement;

use crate::hooks::sound::wasapi::device::MyDevice;

#[implement(IMMDeviceCollection)]
pub(in crate::hooks::sound) struct MyDeviceCollection {
    pub(in crate::hooks::sound) device: Option<IMMDevice>,
}

impl MyDeviceCollection {
    pub(in crate::hooks::sound) fn unique() -> Self {
        let device = Some(MyDevice::new().into());
        Self { device }
    }

    pub(in crate::hooks::sound) fn empty() -> Self {
        let device = None;
        Self { device }
    }
}

#[allow(non_snake_case)]
impl IMMDeviceCollection_Impl for MyDeviceCollection_Impl {
    fn GetCount(&self) -> windows_result::Result<u32> {
        log::trace!("MyDeviceCollection GetCount");
        Ok(self.device.as_slice().len() as _)
    }

    fn Item(&self, device_num: u32) -> windows_result::Result<IMMDevice> {
        log::trace!("MyDeviceCollection Item");
        Ok(self
            .device
            .as_slice()
            .get(device_num as usize)
            .cloned()
            .ok_or(E_INVALIDARG)?)
    }
}
