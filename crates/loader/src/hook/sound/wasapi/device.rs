use windows::Win32::{
    Foundation::E_NOINTERFACE,
    Media::Audio::{
        DEVICE_STATE,
        DEVICE_STATE_ACTIVE,
        IAudioClient,
        IMMDevice,
        IMMDevice_Impl,
    },
    System::Com::{
        CLSCTX,
        CoTaskMemAlloc,
        STGM,
        STGM_READ,
        StructuredStorage::PROPVARIANT,
    },
    UI::Shell::PropertiesSystem::IPropertyStore,
};
use windows_core::{
    GUID,
    Interface,
    imp::E_INVALIDARG,
    implement,
};
use windows_strings::{
    PWSTR,
    w,
};

use crate::hook::sound::wasapi::{
    audio_client::MyAudioClient,
    property_store::MyDevicePropertyStore,
};

#[implement(IMMDevice)]
pub(in crate::hook::sound) struct MyDevice;

impl MyDevice {
    pub fn new() -> Self {
        Self
    }
}

#[allow(non_snake_case)]
impl IMMDevice_Impl for MyDevice_Impl {
    fn Activate(
        &self,
        iid: *const GUID,
        _cls_ctx: CLSCTX,
        _activation_params: *const PROPVARIANT,
        out_interface: *mut *mut std::ffi::c_void,
    ) -> windows_result::Result<()> {
        unsafe {
            let iid = *iid;
            match iid {
                IAudioClient::IID => {
                    log::trace!("MyDevice Activate IAudioClient");
                    let audio_client: IAudioClient = MyAudioClient::new().into();
                    *out_interface = audio_client.into_raw();
                    Ok(())
                }
                _ => {
                    log::trace!("MyDevice Activate {iid:?}");
                    Err(E_NOINTERFACE)?
                }
            }
        }
    }

    fn OpenPropertyStore(&self, stgm_access: STGM) -> windows_result::Result<IPropertyStore> {
        log::trace!("MyDevice OpenPropertyStore");
        if stgm_access != STGM_READ {
            Err(E_INVALIDARG)?
        }
        Ok(MyDevicePropertyStore::new().into())
    }

    fn GetId(&self) -> windows_result::Result<PWSTR> {
        log::trace!("MyDevice GetId");
        unsafe {
            let id = w!("810");
            let size = (id.len() + 1) * size_of::<u16>();
            let p = CoTaskMemAlloc(size);
            let s = std::slice::from_raw_parts_mut(p.cast(), size / size_of::<u16>());
            let (z, s) = s.split_last_mut().unwrap();
            s.copy_from_slice(id.as_wide());
            *z = 0;
            Ok(PWSTR(p as *mut u16))
        }
    }

    fn GetState(&self) -> windows_result::Result<DEVICE_STATE> {
        log::trace!("MyDevice GetState");
        Ok(DEVICE_STATE_ACTIVE)
    }
}
