use windows::Win32::{
    Foundation::ERROR_NOT_FOUND,
    Media::Audio::{
        DEVICE_STATE,
        DEVICE_STATE_ACTIVE,
        EDataFlow,
        ERole,
        IMMDevice,
        IMMDeviceCollection,
        IMMDeviceEnumerator,
        IMMDeviceEnumerator_Impl,
        IMMNotificationClient,
        eConsole,
        eRender,
    },
};
use windows_core::{
    Ref,
    implement,
};
use windows_strings::{
    PCWSTR,
    w,
};

use crate::hooks::sound::wasapi::{
    device::MyDevice,
    device_collection::MyDeviceCollection,
};

#[implement(IMMDeviceEnumerator)]
pub(in crate::hooks::sound) struct MyDeviceEnumerator;

impl MyDeviceEnumerator {
    pub(in crate::hooks::sound) fn new() -> Self {
        Self
    }
}

#[allow(non_snake_case)]
impl IMMDeviceEnumerator_Impl for MyDeviceEnumerator_Impl {
    fn EnumAudioEndpoints(
        &self,
        dataflow: EDataFlow,
        state_mask: DEVICE_STATE,
    ) -> windows_result::Result<IMMDeviceCollection> {
        log::trace!("MyDeviceEnumerator EnumAudioEndpoints");
        let r = if dataflow == eRender && state_mask.0 & DEVICE_STATE_ACTIVE.0 != 0 {
            MyDeviceCollection::unique()
        } else {
            MyDeviceCollection::empty()
        };
        Ok(r.into())
    }

    fn GetDefaultAudioEndpoint(
        &self,
        dataflow: EDataFlow,
        role: ERole,
    ) -> windows_result::Result<IMMDevice> {
        log::trace!("MyDeviceEnumerator GetDefaultAudioEndpoint");
        if dataflow != eRender || role != eConsole {
            Err(ERROR_NOT_FOUND)?;
        }
        Ok(MyDevice::new().into())
    }

    fn GetDevice(&self, id: &PCWSTR) -> windows_result::Result<IMMDevice> {
        log::trace!("MyDeviceEnumerator GetDevice");
        unsafe {
            if id.as_wide() != w!("810").as_wide() {
                Err(ERROR_NOT_FOUND)?
            }
        }
        Ok(MyDevice::new().into())
    }

    fn RegisterEndpointNotificationCallback(
        &self,
        _client: Ref<IMMNotificationClient>,
    ) -> windows_result::Result<()> {
        log::trace!("MyDeviceEnumerator RegisterEndpointNotificationCallback");
        Ok(())
    }

    fn UnregisterEndpointNotificationCallback(
        &self,
        _client: Ref<IMMNotificationClient>,
    ) -> windows_result::Result<()> {
        log::trace!("MyDeviceEnumerator UnregisterEndpointNotificationCallback");
        Ok(())
    }
}
