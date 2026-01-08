use std::ops::ControlFlow;

use windows_sys::Win32::System::Com::CoCreateInstance;

use crate::hook::sound;

#[recordin_macro::static_hook]
#[allow(dead_code)]
pub(super) unsafe extern "system" fn CoCreateInstance(
    cls_id: *const windows_sys::core::GUID,
    outer: *mut core::ffi::c_void,
    cls_context: windows_sys::Win32::System::Com::CLSCTX,
    iid: *const windows_sys::core::GUID,
    out_v: *mut *mut core::ffi::c_void,
) -> windows_sys::core::HRESULT {
    let chain = || {
        sound::com_hook(cls_id, outer, cls_context, iid, out_v)?;
        ControlFlow::Continue(())
    };
    if let ControlFlow::Break(hr) = chain() {
        return hr;
    }
    // unsafe {
    //     let cls = *cls_id;
    //     let cls = GUID::from_values(cls.data1, cls.data2, cls.data3, cls.data4);
    //     let interface = *iid;
    //     let interface = GUID::from_values(
    //         interface.data1,
    //         interface.data2,
    //         interface.data3,
    //         interface.data4,
    //     );
    //     log::trace!("CoCreateInstance {:?} {:?}", cls, interface);
    // }
    unsafe { orig_CoCreateInstance(cls_id, outer, cls_context, iid, out_v) }
}

pub(super) fn init() -> anyhow::Result<()> {
    unsafe {
        init_CoCreateInstance(CoCreateInstance)?.enable()?;
    }
    Ok(())
}
