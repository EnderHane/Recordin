use std::{
    ops::ControlFlow,
    path::Path,
};

use libloading::os::windows::Library;

mod factory;
mod swap_chain;

pub(super) fn lib_load_hook(filename: &str, module: usize) -> ControlFlow<anyhow::Result<()>> {
    let path: &Path = filename.as_ref();
    let name = path.file_stem().unwrap();
    if name == "dxgi" {
        log::trace!("LoadLibrary dxgi.dll: {}", filename);
        unsafe {
            let lib = Library::from_raw(module as _);
            if let Some(r) = init(&lib) {
                lib.into_raw();
                ControlFlow::Break(r)?
            }
        }
    }
    ControlFlow::Continue(())
}

pub(super) fn init_early_loaded() -> Option<anyhow::Result<usize>> {
    let lib = Library::open_already_loaded("dxgi").ok()?;
    log::trace!("LdrLoadDll dxgi.dll");
    let r = init(&lib)?;
    Some(r.map(|_| lib.into_raw() as usize))
}

fn init(lib: &Library) -> Option<anyhow::Result<()>> {
    #[allow(non_snake_case)]
    unsafe {
        let pfn_CreateDXGIFactory = *lib.get("CreateDXGIFactory").ok()?;
        let pfn_CreateDXGIFactory1 = *lib.get("CreateDXGIFactory1").ok()?;
        // let pfn_CreateDXGIFactory2 = *lib.get("CreateDXGIFactory2").ok()?;
        let a = || {
            factory::init_CreateDXGIFactory(pfn_CreateDXGIFactory)?.enable()?;
            factory::init_CreateDXGIFactory1(pfn_CreateDXGIFactory1)?.enable()?;
            Ok(())
        };
        Some(a())
    }
}
