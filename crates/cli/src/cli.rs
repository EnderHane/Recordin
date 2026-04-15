use std::{
    path::Path,
    process::Command,
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

#[derive(Debug, Clone, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[clap(short = 'r', long, help = "Desired FPS for program")]
    pub fps: f64,
    #[clap(flatten)]
    pub graphics: Graphics,
    #[clap(flatten)]
    pub sound: Sound,
    #[clap(alias = "venc", long, help = "Video encoder FFmpeg uses")]
    pub video_encoder: Option<String>,
    #[clap(alias = "vopt", long, help = "Video encoder options")]
    pub video_option: Option<String>,
    #[clap(short = 'v', long, help = "Path of video output file")]
    pub video_output: Option<String>,
    #[clap(short = 'a', long, help = "Path of audio output file")]
    pub audio_output: Option<String>,
    #[clap(short = 'R', long = "")]
    pub target_regex: Option<String>,
    #[clap(short = 'I', long = "aggressive")]
    pub aggressive_infect: bool,
    #[clap(short = 'T', long = "force-tick")]
    pub force_tick_threshold: Option<u64>,
    #[clap(help = "Path of executable to start")]
    pub executable: String,
    #[clap(last = true, help = "Arguments passed to executable")]
    pub exec_args: Vec<String>,
}

#[derive(Debug, Clone, clap::Args, Default)]
#[group(required = false, multiple = false)]
pub struct Graphics {
    #[clap(alias = "vk", long, help = "Hack Vulkan API")]
    pub vulkan: bool,
    #[clap(alias = "d3d", long, help = "Hack Direct3D API")]
    pub d3d11: bool,
}

#[derive(Debug, Clone, clap::Args, Default)]
#[group(required = false, multiple = false)]
pub struct Sound {
    #[clap(long, help = "Hack WASAPI")]
    pub wasapi: bool,
}

pub fn run(cli: Cli) -> color_eyre::Result<()> {
    unsafe {
        std::env::set_var(ENV_KEY_IS_CLI, "1");
    }

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

    let loader_module = unsafe { libloading::Library::new("recordin") }?;
    let magic_symbol: libloading::Symbol<*const u64> = unsafe { loader_module.get("__MAGIC__") }?;
    assert_eq!(
        unsafe { **magic_symbol },
        1145141919810,
        "unexpected DLL loaded"
    );

    if let Some(ffmpeg_encoder) = &cli.video_encoder {
        unsafe {
            std::env::set_var(ENV_KEY_VIDEO_ENCODER, ffmpeg_encoder);
            if let Some(ffmpeg_args) = &cli.video_option {
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
    if cli.graphics.vulkan {
        println!("Vulkan enabled");
        unsafe {
            std::env::set_var(ENV_KEY_GRAPHICS_SYSTEM, "Vulkan");
        }
    } else if cli.graphics.d3d11 {
        println!("D3D11 enabled");
        unsafe {
            std::env::set_var(ENV_KEY_GRAPHICS_SYSTEM, "D3D11");
        }
    }
    if cli.sound.wasapi {
        println!("WASAPI enabled");
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
