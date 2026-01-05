use crate::hook::timing;

#[recordin_macro::static_hook]
#[allow(dead_code)]
pub(super) unsafe extern "system" fn timeGetTime() -> u32 {
    let (pc, f) = timing::perf();
    (pc / f * 1000) as _
}
