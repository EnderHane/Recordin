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
    #[clap(short = 'A', long, help = "Merge audio to video")]
    pub merge_audio: bool,
    #[clap(short = 'R', long, help = "")]
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

#[derive(Debug, Clone, clap::Args)]
#[group(required = false, multiple = false)]
pub struct Graphics {
    #[clap(alias = "vk", long, help = "Hack Vulkan API")]
    pub vulkan: bool,
    #[clap(alias = "d3d", long, help = "Hack Direct3D API")]
    pub d3d11: bool,
}

#[derive(Debug, Clone, clap::Args)]
#[group(required = false, multiple = false)]
pub struct Sound {
    #[clap(long, help = "Hack WASAPI")]
    pub wasapi: bool,
}
