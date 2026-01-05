use std::{
    cell::Cell,
    sync::atomic::{
        AtomicBool,
        AtomicI64,
        AtomicU64,
        Ordering,
    },
};

use windows_sys::Win32::{
    Foundation::TRUE,
    Media::timeGetTime,
    System::{
        Performance::{
            QueryPerformanceCounter,
            QueryPerformanceFrequency,
        },
        SystemInformation::{
            GetTickCount,
            GetTickCount64,
        },
        Threading::{
            Sleep,
            WaitForMultipleObjects,
            WaitForSingleObject,
        },
    },
};

use crate::env;

mod get_tick_count;
mod sleep;
mod sync;
mod time_get_time;

static BASE_COUNT: AtomicI64 = AtomicI64::new(0);
static OFFSET: AtomicI64 = AtomicI64::new(0);

pub(super) static TICK: AtomicI64 = AtomicI64::new(0);
pub(super) static ENABLED: AtomicBool = AtomicBool::new(false);

static ALARM: AtomicU64 = AtomicU64::new(0);

pub fn perf() -> (i64, i64) {
    let (mut pc, mut f) = (0, 0);
    unsafe {
        QueryPerformanceCounter(&mut pc);
        QueryPerformanceFrequency(&mut f);
    }
    (pc, f)
}

pub fn real() -> (i64, i64) {
    let (mut pc, mut f) = (0, 0);
    unsafe {
        orig_QueryPerformanceCounter(&mut pc);
        orig_QueryPerformanceFrequency(&mut f);
    }
    (pc, f)
}

pub(super) fn incr_tick() {
    if !ENABLED.load(Ordering::Acquire) {
        ENABLED.store(true, Ordering::Release);
    } else {
        ALARM.store(0, Ordering::Relaxed);
        TICK.fetch_add(1, Ordering::Relaxed);
        sync::tick();
    }
}

pub(super) fn pause() {
    if ENABLED.load(Ordering::Acquire) {
        sync::tick();
        let c = real().0;
        let offset = BASE_COUNT.load(Ordering::Relaxed) - c;
        OFFSET.store(offset, Ordering::Relaxed);
        ENABLED.store(false, Ordering::Release);
    }
}

std::thread_local! {
    static FORCE_TICK_THRESHOLD: Cell<u64> =
        Cell::new(env::FORCE_TICK_THRESHOLD.as_ref().copied().unwrap_or(65536));

    static COUNT_PER_TICK: Cell<i64> = Cell::new({
        let f = real().1;
        let fps = env::FPS.get();
        let count_per_frame = f as f64 / fps;
        count_per_frame.round() as i64
    });

    static MSPF: Cell<f64> = Cell::new(env::FPS.get().recip() * 1000.);
}

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn QueryPerformanceCounter(p_count: *mut i64) -> windows_sys::core::BOOL {
    if ENABLED.load(Ordering::Acquire) {
        if ALARM.fetch_add(1, Ordering::Relaxed) >= FORCE_TICK_THRESHOLD.get() {
            ALARM.store(0, Ordering::Relaxed);
            log::trace!("Alarm");
            incr_tick();
        }
        unsafe {
            *p_count = BASE_COUNT.load(Ordering::Relaxed)
                + TICK.load(Ordering::Relaxed) * COUNT_PER_TICK.get();
        }
    } else {
        unsafe {
            let c = real().0;
            *p_count = c
                + OFFSET.load(Ordering::Relaxed)
                + TICK.load(Ordering::Relaxed) * COUNT_PER_TICK.get();
            BASE_COUNT.store(c, Ordering::Relaxed);
        }
    }
    TRUE
}

#[recordin_macro::static_hook]
#[allow(dead_code)]
unsafe extern "system" fn QueryPerformanceFrequency(p_freq: *mut i64) -> windows_sys::core::BOOL {
    unsafe {
        std::thread_local! {
            static FREQ: Cell<i64> = Cell::new(real().1);
        }
        *p_freq = FREQ.get();
    }
    TRUE
}

pub(super) fn init() -> anyhow::Result<()> {
    BASE_COUNT.store(perf().0, Ordering::Relaxed);
    unsafe {
        init_QueryPerformanceFrequency(QueryPerformanceFrequency)?.enable()?;
        init_QueryPerformanceCounter(QueryPerformanceCounter)?.enable()?;
        get_tick_count::init_GetTickCount(GetTickCount)?.enable()?;
        get_tick_count::init_GetTickCount64(GetTickCount64)?.enable()?;
        time_get_time::init_timeGetTime(timeGetTime)?.enable()?;
        sleep::init_Sleep(Sleep)?.enable()?;
        sync::init_WaitForSingleObject(WaitForSingleObject)?.enable()?;
        sync::init_WaitForMultipleObjects(WaitForMultipleObjects)?.enable()?;
    }
    Ok(())
}
