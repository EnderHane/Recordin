use std::{
    collections::BTreeMap,
    convert::Into,
    ffi::OsString,
    os::windows::ffi::OsStringExt,
    path::PathBuf,
    string::ToString,
    sync::LazyLock,
};

use recordin_common::{
    ENV_KEY_AGGRESSIVE,
    ENV_KEY_ALLOC_CONSOLE,
    ENV_KEY_AUDIO_OUTPUT,
    ENV_KEY_FORCE_TICK_THRES,
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
use windows_sys::{
    Win32::System::Environment::GetCommandLineW,
    core::{
        PCSTR,
        PCWSTR,
    },
};

pub static FORCE_TICK_THRESHOLD: LazyLock<Option<u64>> = LazyLock::new(|| {
    let a = std::env::var_os(ENV_KEY_FORCE_TICK_THRES)?;
    u64::from_str_radix(&a.to_string_lossy(), 16).ok()
});

pub static GRAPHICS_SYSTEM: LazyLock<Option<String>> = LazyLock::new(|| {
    let e = std::env::var_os(ENV_KEY_GRAPHICS_SYSTEM)?;
    Some(e.to_string_lossy().to_string().to_lowercase())
});

pub static VIDEO_ENCODER: LazyLock<Option<String>> = LazyLock::new(|| {
    let a = std::env::var_os(ENV_KEY_VIDEO_ENCODER)?;
    Some(a.to_string_lossy().to_string())
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

pub static VIDEO_OUTPUT: LazyLock<Option<String>> = LazyLock::new(|| {
    let e = std::env::var_os(ENV_KEY_VIDEO_OUTPUT)?;
    Some(e.to_string_lossy().to_string().replace("{p}", &this_exe()))
});

pub static AUDIO_OUTPUT: LazyLock<Option<String>> = LazyLock::new(|| {
    let e = std::env::var_os(ENV_KEY_AUDIO_OUTPUT)?;
    Some(e.to_string_lossy().to_string().replace("{p}", &this_exe()))
});

pub static FPS: LazyLock<Option<f64>> = LazyLock::new(|| {
    let fps_str = std::env::var(ENV_KEY_FPS_F64_HEX)
        .inspect_err(|e| log::warn!("unable to determine FPS: {e}"))
        .ok()?;
    u64::from_str_radix(&fps_str, 16)
        .inspect_err(|e| log::warn!("unable to determine FPS: {e}"))
        .map(f64::from_bits)
        .inspect(|fps| log::info!("Desired FPS: {fps}"))
        .ok()
});

pub fn should_emit_video() -> bool {
    VIDEO_ENCODER.is_some() && FPS.is_some()
}

pub static CMDLINE: LazyLock<OsString> = LazyLock::new(|| {
    let p_cmdline = unsafe { GetCommandLineW() };
    unsafe { OsString::from_wide(windows_strings::PCWSTR::from_raw(p_cmdline).as_wide()) }
});

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

pub unsafe fn should_inject_a(p_app_name: PCSTR, p_cmdline: PCSTR) -> bool {
    let Some(re) = TARGET_REGEX.as_ref() else {
        return false;
    };
    unsafe {
        if !p_app_name.is_null() {
            let Ok(name) = windows_strings::PCSTR::from_raw(p_app_name).to_string() else {
                return false;
            };
            re.is_match(&name)
        } else if !p_cmdline.is_null() {
            let Ok(cmd) = windows_strings::PCSTR::from_raw(p_cmdline).to_string() else {
                return false;
            };
            let argv = winsplit::split(&cmd);
            argv.first().is_some_and(|s| re.is_match(s))
        } else {
            false
        }
    }
}

pub unsafe fn should_inject_w(p_app_name: PCWSTR, p_cmdline: PCWSTR) -> bool {
    let Some(re) = TARGET_REGEX.as_ref() else {
        return false;
    };
    unsafe {
        if !p_app_name.is_null() {
            let Ok(name) = windows_strings::PCWSTR::from_raw(p_app_name).to_string() else {
                return false;
            };
            re.is_match(&name)
        } else if !p_cmdline.is_null() {
            let Ok(cmd) = windows_strings::PCWSTR::from_raw(p_cmdline).to_string() else {
                return false;
            };
            let argv = winsplit::split(&cmd);
            argv.first().is_some_and(|s| re.is_match(s))
        } else {
            false
        }
    }
}

pub fn this_exe() -> String {
    let cmd = CMDLINE.to_string_lossy();
    let argv = winsplit::split(&cmd);
    let path = PathBuf::from(&argv[0]);
    path.file_name().unwrap().to_string_lossy().to_string()
}
