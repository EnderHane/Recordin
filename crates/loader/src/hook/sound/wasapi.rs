use std::ops::ControlFlow;

use device_enumerator::MyDeviceEnumerator;
use windows::Win32::Media::Audio::IMMDeviceEnumerator;
use windows_core::{
    GUID,
    Interface,
};
use windows_sys::Win32::Foundation::S_OK;

mod audio_client;
mod audio_render_client;
mod device;
mod device_collection;
mod device_enumerator;
mod property_store;

pub fn com_hook(
    cls_id: *const windows_sys::core::GUID,
    _outer: *mut core::ffi::c_void,
    _cls_context: windows_sys::Win32::System::Com::CLSCTX,
    iid: *const windows_sys::core::GUID,
    out_v: *mut *mut core::ffi::c_void,
) -> ControlFlow<windows_sys::core::HRESULT> {
    unsafe {
        let cls = *cls_id;
        let _cls = GUID::from_values(cls.data1, cls.data2, cls.data3, cls.data4);
        let intf = *iid;
        let intf = GUID::from_values(intf.data1, intf.data2, intf.data3, intf.data4);
        if intf == IMMDeviceEnumerator::IID {
            log::trace!("CoCreateInstance IMMDeviceEnumerator");
            let my: IMMDeviceEnumerator = MyDeviceEnumerator::new().into();
            *out_v = my.into_raw();
            ControlFlow::Break(S_OK)?;
        }
    }
    ControlFlow::Continue(())
}
