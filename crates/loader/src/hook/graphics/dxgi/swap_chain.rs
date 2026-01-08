use std::{
    cell::OnceCell,
    mem::MaybeUninit,
    sync::atomic::{
        AtomicU64,
        Ordering,
    },
};

use parking_lot::Mutex;
use windows::Win32::Graphics::{
    Direct3D11::{
        D3D11_CPU_ACCESS_READ,
        D3D11_MAP_READ,
        D3D11_USAGE_STAGING,
        ID3D11Device,
        ID3D11DeviceContext,
        ID3D11Texture2D,
    },
    Dxgi::{
        Common::{
            DXGI_FORMAT,
            DXGI_MODE_DESC,
        },
        DXGI_FRAME_STATISTICS,
        DXGI_PRESENT,
        DXGI_SWAP_CHAIN_DESC,
        DXGI_SWAP_CHAIN_FLAG,
        IDXGIDeviceSubObject,
        IDXGIDeviceSubObject_Impl,
        IDXGIObject,
        IDXGIObject_Impl,
        IDXGIOutput,
        IDXGISwapChain,
        IDXGISwapChain_Impl,
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
use windows_result::{
    BOOL,
    HRESULT,
};

use crate::{
    hook::{
        graphics,
        timing,
    },
    output::{
        video_codec,
        video_codec::EncDuplex,
    },
};

#[implement(IDXGISwapChain)]
pub(super) struct MyDXGISwapChain {
    inner: IDXGISwapChain,
    frame_count: AtomicU64,
    init_real_time: i64,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    present_state: Mutex<OnceCell<PresentState>>,
}

struct PresentState {
    present_image: ID3D11Texture2D,
    image: ID3D11Texture2D,
    width: usize,
    height: usize,
    encoder: Option<EncDuplex>,
}

impl MyDXGISwapChain {
    pub(super) fn new(inner: IDXGISwapChain) -> Result<Self, IDXGISwapChain> {
        unsafe {
            let Ok(device) = inner.GetDevice::<ID3D11Device>().inspect_err(|e| {
                log::debug!(
                    "DXGI swap chain created but not on Direct3D 11 device: {}",
                    e
                );
            }) else {
                Err(inner)?
            };
            log::debug!("ID3D11Device@{device:?} create IDXGISwapChain@{inner:?}");
            let context = device
                .GetImmediateContext()
                .expect("device should have immediate context");
            let r = Self {
                inner,
                frame_count: AtomicU64::new(0),
                init_real_time: timing::real().0,
                device,
                context,
                present_state: Mutex::new(OnceCell::new()),
            };
            Ok(r)
        }
    }
}

impl Drop for MyDXGISwapChain {
    fn drop(&mut self) {
        log::debug!("MyDXGISwapChain drop");
        let fr = self.frame_count.load(Ordering::Relaxed) as f64;
        let (t, f) = timing::real();
        let dt = (t - self.init_real_time) as f64;
        let in_sec = dt / f as f64;
        let time: humantime::Duration = std::time::Duration::from_secs_f64(in_sec).into();
        let fps = fr / in_sec;
        log::debug!("Frames: {fr}, Real Time: {time}, Average FPS: {fps:0.2},");
        timing::pause();
    }
}

#[allow(non_snake_case)]
impl IDXGISwapChain_Impl for MyDXGISwapChain_Impl {
    fn Present(&self, sync_interval: u32, flags: DXGI_PRESENT) -> HRESULT {
        // log::trace!("MyDXGISwapChain Present");
        self.frame_count.fetch_add(1, Ordering::Relaxed);
        let present_lock = self.present_state.lock();
        let PresentState {
            present_image,
            image,
            width,
            height,
            encoder,
        } = present_lock.get_or_init(|| unsafe {
            let present_image: ID3D11Texture2D = self.inner.GetBuffer(0).unwrap();
            let mut image_desc = MaybeUninit::zeroed();
            present_image.GetDesc(image_desc.as_mut_ptr());
            let mut image_desc = image_desc.assume_init();
            let width = image_desc.Width as usize;
            let height = image_desc.Height as usize;
            image_desc.MipLevels = 1;
            image_desc.ArraySize = 1;
            image_desc.SampleDesc.Count = 1;
            image_desc.SampleDesc.Quality = 0;
            image_desc.Usage = D3D11_USAGE_STAGING;
            image_desc.BindFlags = 0;
            image_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as _;
            image_desc.MiscFlags = 0;
            let mut image = None;
            self.device
                .CreateTexture2D(&image_desc, None, Some(&mut image))
                .unwrap();
            let image = image.unwrap();
            let encoder = video_codec::create_encoder(width, height);
            PresentState {
                present_image,
                image,
                width,
                height,
                encoder,
            }
        });
        unsafe {
            self.context.CopyResource(image, present_image);
            let mut map_res = MaybeUninit::zeroed();
            self.context
                .Map(image, 0, D3D11_MAP_READ, 0, Some(map_res.as_mut_ptr()))
                .unwrap();
            let map_res = map_res.assume_init();
            let mapped = map_res.pData;
            let row_pitch = map_res.RowPitch;
            if let Some((tx, rx)) = encoder
                && let Ok(mut packed_bgr) = rx.recv()
            {
                packed_bgr.resize(width * height, [0; _]);
                let packed_lines = packed_bgr.chunks_exact_mut(*width);
                let mapped_slices = graphics::slices_by_row_pitch(
                    mapped.cast(),
                    width * 4,
                    *height,
                    row_pitch as usize,
                );
                let z = packed_lines.zip(mapped_slices);
                for (packed_line, mapped_slice) in z {
                    let (raw_c, _) = mapped_slice.as_chunks();
                    let zz = packed_line.iter_mut().zip(raw_c);
                    for (packed, &[b, g, r, _]) in zz {
                        *packed = [b, g, r];
                    }
                }
                tx.send(packed_bgr).ok();
            }
            self.context.Unmap(image, 0);
        }
        timing::incr_tick();
        unsafe { self.inner.Present(sync_interval, flags) }
    }

    fn GetBuffer(
        &self,
        buffer: u32,
        iid: *const GUID,
        out_surface: *mut *mut core::ffi::c_void,
    ) -> windows_result::Result<()> {
        // log::trace!("MyDXGISwapChain GetBuffer");
        let o: &IDXGISwapChain = &self.inner;
        unsafe { (o.vtable().GetBuffer)(o.as_raw(), buffer, iid, out_surface).ok() }
    }

    fn SetFullscreenState(
        &self,
        fullscreen: BOOL,
        target: Ref<IDXGIOutput>,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGISwapChain SetFullscreenState");
        unsafe {
            self.inner
                .SetFullscreenState(fullscreen.as_bool(), target.as_ref())
        }
    }

    fn GetFullscreenState(
        &self,
        out_fullscreen: *mut BOOL,
        out_target: OutRef<IDXGIOutput>,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGISwapChain GetFullscreenState");
        unsafe {
            let mut target = None;
            self.inner
                .GetFullscreenState(out_fullscreen.into(), Some(&mut target))?;
            out_target.write(target)?;
            Ok(())
        }
    }

    fn GetDesc(&self) -> windows_result::Result<DXGI_SWAP_CHAIN_DESC> {
        log::trace!("MyDXGISwapChain GetDesc");
        unsafe { self.inner.GetDesc() }
    }

    fn ResizeBuffers(
        &self,
        buffer_count: u32,
        width: u32,
        height: u32,
        new_format: DXGI_FORMAT,
        swap_chain_flags: &DXGI_SWAP_CHAIN_FLAG,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGISwapChain ResizeBuffers");
        {
            self.present_state.lock().take();
        }
        unsafe {
            self.inner
                .ResizeBuffers(buffer_count, width, height, new_format, *swap_chain_flags)
        }
    }

    fn ResizeTarget(
        &self,
        new_target_parameters: *const DXGI_MODE_DESC,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGISwapChain ResizeTarget");
        unsafe { self.inner.ResizeTarget(new_target_parameters) }
    }

    fn GetContainingOutput(&self) -> windows_result::Result<IDXGIOutput> {
        log::trace!("MyDXGISwapChain GetContainingOutput");
        unsafe { self.inner.GetContainingOutput() }
    }

    fn GetFrameStatistics(
        &self,
        out_stats: *mut DXGI_FRAME_STATISTICS,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGISwapChain GetFrameStatistics");
        unsafe { self.inner.GetFrameStatistics(out_stats) }
    }

    fn GetLastPresentCount(&self) -> windows_result::Result<u32> {
        log::trace!("MyDXGISwapChain GetLastPresentCount");
        unsafe { self.inner.GetLastPresentCount() }
    }
}

#[allow(non_snake_case)]
impl IDXGIDeviceSubObject_Impl for MyDXGISwapChain_Impl {
    fn GetDevice(
        &self,
        iid: *const GUID,
        out_device: *mut *mut core::ffi::c_void,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGISwapChain GetDevice");
        let o: &IDXGIDeviceSubObject = &self.inner;
        unsafe { (o.vtable().GetDevice)(o.as_raw(), iid, out_device).ok() }
    }
}

#[allow(non_snake_case)]
impl IDXGIObject_Impl for MyDXGISwapChain_Impl {
    fn SetPrivateData(
        &self,
        name: *const GUID,
        size: u32,
        data: *const core::ffi::c_void,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGISwapChain SetPrivateData");
        unsafe { self.inner.SetPrivateData(name, size, data) }
    }

    fn SetPrivateDataInterface(
        &self,
        name: *const GUID,
        interface: Ref<IUnknown>,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGISwapChain SetPrivateDataInterface");
        unsafe { self.inner.SetPrivateDataInterface(name, interface.as_ref()) }
    }

    fn GetPrivateData(
        &self,
        name: *const GUID,
        size: *mut u32,
        data: *mut core::ffi::c_void,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGISwapChain GetPrivateData");
        unsafe { self.inner.GetPrivateData(name, size, data) }
    }

    fn GetParent(
        &self,
        iid: *const GUID,
        out_parent: *mut *mut core::ffi::c_void,
    ) -> windows_result::Result<()> {
        log::trace!("MyDXGISwapChain GetParent");
        let o: &IDXGIObject = &self.inner;
        unsafe { (o.vtable().GetParent)(o.as_raw(), iid, out_parent).ok() }
    }
}
