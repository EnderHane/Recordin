use windows::Win32::{
    Foundation::{
        HMODULE,
        HWND,
    },
    Graphics::Dxgi::{
        DXGI_MWA_FLAGS,
        DXGI_SWAP_CHAIN_DESC,
        IDXGIAdapter,
        IDXGIAdapter1,
        IDXGIFactory,
        IDXGIFactory_Impl,
        IDXGIFactory1,
        IDXGIFactory1_Impl,
        IDXGIObject,
        IDXGIObject_Impl,
        IDXGISwapChain,
    },
};
use windows_core::{
    GUID,
    IUnknown,
    Interface,
    OutRef,
    Ref,
    implement,
};
use windows_result::BOOL;
use windows_sys::{
    Win32::Foundation::{
        E_NOINTERFACE,
        S_OK,
    },
    core::HRESULT,
};

use crate::hook::graphics::dxgi::swap_chain::MyDXGISwapChain;

#[recordin_macro::static_hook]
pub(in super::super) unsafe extern "system" fn CreateDXGIFactory(
    iid: *const windows_sys::core::GUID,
    out_factory: *mut *mut core::ffi::c_void,
) -> HRESULT {
    log::trace!("CreateDXGIFactory");
    unsafe { create(iid, out_factory) }
}

#[recordin_macro::static_hook]
pub(in super::super) unsafe extern "system" fn CreateDXGIFactory1(
    iid: *const windows_sys::core::GUID,
    out_factory: *mut *mut core::ffi::c_void,
) -> HRESULT {
    log::trace!("CreateDXGIFactory1");
    unsafe { create(iid, out_factory) }
}

unsafe fn create(
    iid: *const windows_sys::core::GUID,
    out_factory: *mut *mut core::ffi::c_void,
) -> HRESULT {
    unsafe {
        let interface = *iid;
        let interface = GUID::from_values(
            interface.data1,
            interface.data2,
            interface.data3,
            interface.data4,
        );
        match interface {
            IDXGIFactory1::IID | IDXGIFactory::IID => {
                let mut out = std::ptr::null_mut();
                let mut res = orig_CreateDXGIFactory1(
                    &windows_sys::core::GUID::from_u128(IDXGIFactory1::IID.to_u128()),
                    &mut out,
                );
                if res == S_OK {
                    let i = IDXGIFactory1::from_raw(out);
                    let my: IDXGIFactory1 = MyDXGIFactory1::new(i).into();
                    res = my.query(&interface, out_factory).0;
                }
                res
            }
            _ => {
                *out_factory = 0 as _;
                E_NOINTERFACE
            }
        }
    }
}

#[implement(IDXGIFactory1)]
struct MyDXGIFactory1 {
    inner: IDXGIFactory1,
}

impl MyDXGIFactory1 {
    fn new(inner: IDXGIFactory1) -> Self {
        Self { inner }
    }
}

#[allow(non_snake_case)]
impl IDXGIFactory1_Impl for MyDXGIFactory1_Impl {
    fn EnumAdapters1(&self, adapter: u32) -> windows_result::Result<IDXGIAdapter1> {
        log::trace!("MyDXGIFactory1 EnumAdapters1");
        unsafe { self.inner.EnumAdapters1(adapter) }
    }

    fn IsCurrent(&self) -> BOOL {
        log::trace!("MyDXGIFactory1 IsCurrent");
        unsafe { self.inner.IsCurrent() }
    }
}

#[allow(non_snake_case)]
impl IDXGIFactory_Impl for MyDXGIFactory1_Impl {
    fn EnumAdapters(&self, adapter: u32) -> windows_result::Result<IDXGIAdapter> {
        log::trace!("MyDXGIFactory1 EnumAdapters");
        unsafe { self.inner.EnumAdapters(adapter) }
    }

    fn MakeWindowAssociation(
        &self,
        hwnd: HWND,
        flags: DXGI_MWA_FLAGS,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGIFactory1 MakeWindowAssociation");
        unsafe { self.inner.MakeWindowAssociation(hwnd, flags) }
    }

    fn GetWindowAssociation(&self) -> windows_result::Result<HWND> {
        log::trace!("MyDXGIFactory1 GetWindowAssociation");
        unsafe { self.inner.GetWindowAssociation() }
    }

    fn CreateSwapChain(
        &self,
        device: Ref<IUnknown>,
        desc: *const DXGI_SWAP_CHAIN_DESC,
        out_swap_chain: OutRef<IDXGISwapChain>,
    ) -> windows_result::HRESULT {
        log::trace!("MyDXGIFactory1 CreateSwapChain");
        unsafe {
            let mut out = None;
            let mut res = self.inner.CreateSwapChain(device.as_ref(), desc, &mut out);
            if res.is_ok()
                && let Some(o) = out
            {
                let maybe: IDXGISwapChain =
                    MyDXGISwapChain::new(o).map_or_else(|a| a, |b| b.into());
                res = out_swap_chain
                    .write(Some(maybe))
                    .map_or_else(|e| e.code(), |_| windows_core::HRESULT(S_OK));
            }
            res
        }
    }

    fn CreateSoftwareAdapter(&self, module: HMODULE) -> windows_result::Result<IDXGIAdapter> {
        log::trace!("MyDXGIFactory1 CreateSoftwareAdapter");
        unsafe { self.inner.CreateSoftwareAdapter(module) }
    }
}

#[allow(non_snake_case)]
impl IDXGIObject_Impl for MyDXGIFactory1_Impl {
    fn SetPrivateData(
        &self,
        name: *const GUID,
        size: u32,
        data: *const core::ffi::c_void,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGIFactory1 SetPrivateData");
        unsafe { self.inner.SetPrivateData(name, size, data) }
    }

    fn SetPrivateDataInterface(
        &self,
        name: *const GUID,
        interface: Ref<IUnknown>,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGIFactory1 SetPrivateDataInterface");
        unsafe { self.inner.SetPrivateDataInterface(name, interface.as_ref()) }
    }

    fn GetPrivateData(
        &self,
        name: *const GUID,
        size: *mut u32,
        data: *mut core::ffi::c_void,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGIFactory1 GetPrivateData");
        unsafe { self.inner.GetPrivateData(name, size, data) }
    }

    fn GetParent(
        &self,
        iid: *const GUID,
        out_parent: *mut *mut core::ffi::c_void,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGIFactory1 GetParent");
        let o: &IDXGIObject = &self.inner;
        unsafe { (o.vtable().GetParent)(o.as_raw(), iid, out_parent).ok() }
    }
}
