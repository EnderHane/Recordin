use std::{
    path::Path,
    process::Command,
};

use libloading::{
    Library,
    Symbol,
};
use recordin_common::{
    ENV_KEY_AGGRESSIVE,
    ENV_KEY_AUDIO_OUTPUT,
    ENV_KEY_FORCE_TICK_THRESHOLD,
    ENV_KEY_FPS_F64_HEX,
    ENV_KEY_GRAPHICS_SYSTEM,
    ENV_KEY_IS_CLI,
    ENV_KEY_SOUND_SYSTEM,
    ENV_KEY_TARGET_REGEX,
    ENV_KEY_VIDEO_ARGS,
    ENV_KEY_VIDEO_ENCODER,
    ENV_KEY_VIDEO_OUTPUT,
};

use crate::cli::Cli;

mod cli;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    unsafe {
        std::env::set_var(ENV_KEY_IS_CLI, "1");
    }
    let cli: Cli = clap::Parser::parse();
    let executable_filename = AsRef::<Path>::as_ref(&cli.executable)
        .file_name()
        .ok_or(color_eyre::eyre::eyre!("Probably invalid executable path"))?
        .to_string_lossy();
    let executable_regex = regex::escape(&executable_filename);
    let target_regex_v = cli.target_regex.as_deref().unwrap_or(&executable_regex);
    if std::env::var_os(ENV_KEY_TARGET_REGEX).is_none() {
        unsafe {
            std::env::set_var(ENV_KEY_TARGET_REGEX, target_regex_v);
        }
    }
    if cli.aggressive_infect {
        unsafe {
            std::env::set_var(ENV_KEY_AGGRESSIVE, "1");
        }
    }
    let mut fps = cli.fps;
    if !(0.5..=3000.0).contains(&fps) {
        fps = 60.0;
    }
    let fps_str = format!("{:x}", fps.to_bits());
    unsafe {
        std::env::set_var(ENV_KEY_FPS_F64_HEX, fps_str);
    }
    let loader_module = unsafe { Library::new("recordin_loader") }?;
    let magic_symbol: Symbol<*const u64> = unsafe { loader_module.get("__MAGIC__") }?;
    assert_eq!(
        unsafe { **magic_symbol },
        1145141919810,
        "unexpected DLL loaded"
    );
    if let Some(ffmpeg_encoder) = &cli.video_encoder {
        unsafe {
            std::env::set_var(ENV_KEY_VIDEO_ENCODER, ffmpeg_encoder);
            if let Some(ffmpeg_args) = &cli.video_args {
                std::env::set_var(ENV_KEY_VIDEO_ARGS, ffmpeg_args);
            }
            if let Some(ffmpeg_output) = &cli.video_output {
                std::env::set_var(ENV_KEY_VIDEO_OUTPUT, ffmpeg_output);
            }
        }
    }
    if let Some(audio_output) = &cli.audio_output {
        unsafe {
            std::env::set_var(ENV_KEY_AUDIO_OUTPUT, audio_output);
        }
    }
    if let Some(v) = cli.force_tick_threshold {
        unsafe {
            std::env::set_var(ENV_KEY_FORCE_TICK_THRESHOLD, format!("{v:X}"));
        }
    }
    if cli.vulkan {
        unsafe {
            std::env::set_var(ENV_KEY_GRAPHICS_SYSTEM, "Vulkan");
        }
    }
    if cli.wasapi {
        unsafe {
            std::env::set_var(ENV_KEY_SOUND_SYSTEM, "WASAPI");
        }
    }
    Command::new(&cli.executable)
        .args(&cli.exec_args)
        .env_remove(ENV_KEY_IS_CLI)
        .spawn()?;
    Ok(())
}
