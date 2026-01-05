use std::{
    cell::Cell,
    collections::BTreeMap,
    convert::Into,
    path::PathBuf,
    str::FromStr,
    string::ToString,
    sync::LazyLock,
};

use arrayvec::ArrayString;
use recordin_common::{
    ENV_KEY_AGGRESSIVE,
    ENV_KEY_ALLOC_CONSOLE,
    ENV_KEY_AUDIO_OUTPUT,
    ENV_KEY_FORCE_TICK_THRESHOLD,
    ENV_KEY_FPS_F64_HEX,
    ENV_KEY_GRAPHICS_SYSTEM,
    ENV_KEY_IS_CLI,
    ENV_KEY_LOG_DIR,
    ENV_KEY_SOUND_SYSTEM,
    ENV_KEY_TARGET_REGEX,
    ENV_KEY_VIDEO_ARGS,
    ENV_KEY_VIDEO_ENCODER,
    ENV_KEY_VIDEO_OUTPUT,
};
use regex::{
    Regex,
    RegexBuilder,
};

pub static FORCE_TICK_THRESHOLD: LazyLock<Option<u64>> = LazyLock::new(|| {
    let a = std::env::var_os(ENV_KEY_FORCE_TICK_THRESHOLD)?;
    u64::from_str_radix(&a.to_string_lossy(), 16).ok()
});

pub static GRAPHICS_SYSTEM: LazyLock<Option<String>> = LazyLock::new(|| {
    let e = std::env::var_os(ENV_KEY_GRAPHICS_SYSTEM)?;
    Some(e.to_string_lossy().to_string().to_lowercase())
});

pub static VIDEO_ARGS: LazyLock<Option<BTreeMap<String, String>>> = LazyLock::new(|| {
    let args = std::env::var_os(ENV_KEY_VIDEO_ARGS)?;
    let args = args
        .to_string_lossy()
        .split(';')
        .filter_map(|s| s.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    log::info!("ffmpeg arguments:\n{:?}", args);
    Some(args)
});

pub static SOUND_SYSTEM: LazyLock<Option<String>> = LazyLock::new(|| {
    let e = std::env::var_os(ENV_KEY_SOUND_SYSTEM)?;
    Some(e.to_string_lossy().to_string().to_lowercase())
});

pub static VIDEO_OUTPUT: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let e = std::env::var_os(ENV_KEY_VIDEO_OUTPUT)?;
    Some(PathBuf::from(e))
});

pub static AUDIO_OUTPUT: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let e = std::env::var_os(ENV_KEY_AUDIO_OUTPUT)?;
    Some(PathBuf::from(e))
});

pub fn should_emit_video() -> bool {
    VIDEO_ENCODER.get().is_some()
}

// pub static CMDLINE: LazyLock<OsString> = LazyLock::new(|| {
//     let p_cmdline = unsafe { GetCommandLineW() };
//     unsafe { OsString::from_wide(windows_strings::PCWSTR::from_raw(p_cmdline).as_wide()) }
// });

pub static TARGET_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| {
    let re = std::env::var(ENV_KEY_TARGET_REGEX)
        .inspect_err(|e| log::warn!("Invalid target regular expression: {e}"))
        .ok()?;
    let re = RegexBuilder::new(&re)
        .case_insensitive(true)
        .build()
        .inspect_err(|e| log::warn!("Invalid target regular expression: {e}"))
        .ok()?;
    log::info!("Target regular expression: {re}");
    Some(re)
});

pub static PROCESS_IS_CLI: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os(ENV_KEY_IS_CLI).is_some());

pub static AGGRESSIVE: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os(ENV_KEY_AGGRESSIVE).is_some());

pub static LOG_DIR: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let dir_var: PathBuf = std::env::var_os(ENV_KEY_LOG_DIR)?.into();
    if !dir_var.exists() || dir_var.is_dir() {
        Some(dir_var)
    } else {
        log::warn!("log dir not valid");
        None
    }
});

pub static ALLOC_CONSOLE: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os(ENV_KEY_ALLOC_CONSOLE).is_some());

std::thread_local! {
    pub static VIDEO_ENCODER: Cell<Option<ArrayString<256>>> = {
        let a = std::env::var_os(ENV_KEY_VIDEO_ENCODER)
            .map(|a| ArrayString::from_str(&a.to_string_lossy()).unwrap());
        Cell::new(a)
    };

    pub static FPS: Cell<f64> = {
        let fps = std::env::var(ENV_KEY_FPS_F64_HEX)
            .ok()
            .and_then(|s| {
                u64::from_str_radix(&s, 16).map(f64::from_bits).ok()
            });
        Cell::new(fps.unwrap_or(60.))
    };

    // pub static EXECUTABLE: Cell<ArrayString<256>> = {
    //     let cmd = CMDLINE.to_string_lossy();
    //     let argv = winsplit::split(&cmd);
    //     let path = PathBuf::from(&argv[0]);
    //     let a = ArrayString::from_str(&path.file_name().unwrap().to_string_lossy()).unwrap();
    //     Cell::new(a)
    // };
}

// pub fn this_exe() -> ArrayString<256> {
//     EXECUTABLE.get()
// }
