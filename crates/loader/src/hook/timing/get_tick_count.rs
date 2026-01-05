use crate::hook::timing;

#[recordin_macro::static_hook]
#[allow(dead_code)]
pub(super) unsafe extern "system" fn GetTickCount() -> u32 {
    let (pc, f) = timing::perf();
    (pc / f * 1000) as _
}

#[recordin_macro::static_hook]
#[allow(dead_code)]
pub(super) unsafe extern "system" fn GetTickCount64() -> u64 {
    let (pc, f) = timing::perf();
    (pc / f * 1000) as _
}
