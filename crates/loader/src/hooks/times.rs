use std::sync::atomic::{
    AtomicBool,
    AtomicI64,
    AtomicU64,
    Ordering,
};

use windows_sys::Win32::{
    Foundation::TRUE,
    System::{
        Performance::{
            QueryPerformanceCounter,
            QueryPerformanceFrequency,
        },
        SystemInformation::{
            GetTickCount,
            GetTickCount64,
        },
    },
};

use crate::{
    envs,
    hooks::sound,
};

static PERF_FREQ: AtomicI64 = AtomicI64::new(0);
static BASE_COUNT: AtomicI64 = AtomicI64::new(0);
static OFFSET: AtomicI64 = AtomicI64::new(0);

pub(super) static TICK: AtomicI64 = AtomicI64::new(0);
static ENABLED: AtomicBool = AtomicBool::new(false);
static COUNT_PER_TICK: AtomicI64 = AtomicI64::new(0);

static ALARM: AtomicU64 = AtomicU64::new(0);

pub fn get_perf() -> (i64, i64) {
    let (mut pc, mut f) = (0, 0);
    unsafe {
        QueryPerformanceCounter(&mut pc);
        QueryPerformanceFrequency(&mut f);
    }
    (pc, f)
}

pub(super) fn incr_tick() {
    if !ENABLED.load(Ordering::Acquire) {
        ENABLED.store(true, Ordering::Release);
    }
    ALARM.store(0, Ordering::Relaxed);
    TICK.fetch_add(1, Ordering::Relaxed);
    sound::LISTENER.send(()).ok();
}

pub(super) fn pause() {
    if ENABLED.load(Ordering::Acquire) {
        let mut c = 0;
        unsafe {
            orig_QueryPerformanceCounter(&mut c);
        }
        let offset = BASE_COUNT.load(Ordering::Relaxed) - c;
        OFFSET.store(offset, Ordering::Relaxed);
        ENABLED.store(false, Ordering::Release);
    }
}

static FORCE_TICK_THRESHOLD: AtomicU64 = AtomicU64::new(65536);

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn QueryPerformanceCounter(p_count: *mut i64) -> windows_sys::core::BOOL {
    #[cfg(debug_assertions)]
    {
        static CALL_TIME: AtomicU64 = AtomicU64::new(0);
        let t = CALL_TIME.fetch_add(1, Ordering::Relaxed);
        if t == 1919810 {
            log::trace!("QueryPerformanceCounter x 1919810");
        }
    }
    let mut c = 0;
    unsafe {
        orig_QueryPerformanceCounter(&mut c);
    }
    let current_tick = TICK.load(Ordering::Relaxed);
    let cpf = COUNT_PER_TICK.load(Ordering::Relaxed);
    let emu_elapse = current_tick * cpf;
    if ENABLED.load(Ordering::Acquire) {
        let al = ALARM.fetch_add(1, Ordering::Relaxed);
        let thres = FORCE_TICK_THRESHOLD.load(Ordering::Relaxed);
        if al >= thres {
            ALARM.store(0, Ordering::Relaxed);
            TICK.fetch_add(1, Ordering::Relaxed);
        }
        unsafe {
            *p_count = BASE_COUNT.load(Ordering::Relaxed) + emu_elapse;
        }
    } else {
        unsafe {
            *p_count = c + OFFSET.load(Ordering::Relaxed) + emu_elapse;
            BASE_COUNT.store(c, Ordering::Relaxed);
        }
    }
    TRUE
}

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn QueryPerformanceFrequency(p_freq: *mut i64) -> windows_sys::core::BOOL {
    #[cfg(debug_assertions)]
    {
        static CALL_TIME: AtomicU64 = AtomicU64::new(0);
        let t = CALL_TIME.fetch_add(1, Ordering::Relaxed);
        if t == 1145 {
            log::trace!("QueryPerformanceFrequency x 1145");
        }
    }
    unsafe {
        *p_freq = PERF_FREQ.load(Ordering::Relaxed);
    }
    TRUE
}

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn GetTickCount() -> u32 {
    #[cfg(debug_assertions)]
    {
        static CALL_TIME: AtomicU64 = AtomicU64::new(0);
        let t = CALL_TIME.fetch_add(1, Ordering::Relaxed);
        if t == 1919810 {
            log::trace!("GetTickCount x 1919810");
        }
    }
    let mut f = 0;
    let mut c = 0;
    unsafe {
        QueryPerformanceFrequency(&mut f);
        QueryPerformanceCounter(&mut c);
    }
    (c / f * 1000) as _
}

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn GetTickCount64() -> u64 {
    #[cfg(debug_assertions)]
    {
        static CALL_TIME: AtomicU64 = AtomicU64::new(0);
        let t = CALL_TIME.fetch_add(1, Ordering::Relaxed);
        if t == 1919810 {
            log::trace!("GetTickCount64 x 1919810");
        }
    }
    let mut f = 0;
    let mut c = 0;
    unsafe {
        QueryPerformanceFrequency(&mut f);
        QueryPerformanceCounter(&mut c);
    }
    (c / f * 1000) as _
}

pub(super) fn init() -> anyhow::Result<()> {
    if let Some(&v) = envs::FORCE_TICK_THRESHOLD.as_ref() {
        FORCE_TICK_THRESHOLD.store(v, Ordering::Relaxed);
    }
    let mut freq = 0;
    let mut init = 0;
    unsafe {
        QueryPerformanceFrequency(&mut freq);
        QueryPerformanceCounter(&mut init);
    }
    BASE_COUNT.store(init, Ordering::Relaxed);
    let fps = envs::FPS.unwrap_or(60.);
    PERF_FREQ.store(freq, Ordering::Relaxed);
    let count_per_frame = freq as f64 / fps;
    let cpf = count_per_frame.round() as i64;
    log::trace!("Count per tick: {}", cpf);
    COUNT_PER_TICK.store(cpf, Ordering::Relaxed);
    unsafe {
        init_QueryPerformanceFrequency(QueryPerformanceFrequency)?.enable()?;
        init_QueryPerformanceCounter(QueryPerformanceCounter)?.enable()?;
        init_GetTickCount(GetTickCount)?.enable()?;
        init_GetTickCount64(GetTickCount64)?.enable()?;
    }
    Ok(())
}
