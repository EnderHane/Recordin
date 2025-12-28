#[derive(Debug, Clone, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[clap(short = 'r', long, help = "Desired FPS for program")]
    pub fps: f64,
    #[clap(long, help = "Enable Vulkan")]
    pub vulkan: bool,
    #[clap(long, help = "Enable WASAPI")]
    pub wasapi: bool,
    #[clap(alias = "ve", long, help = "Video encoder FFmpeg uses")]
    pub video_encoder: Option<String>,
    #[clap(alias = "vo", long, help = "Video encoder options")]
    pub video_args: Option<String>,
    #[clap(short = 'v', long, help = "Path of video output file")]
    pub video_output: Option<String>,
    #[clap(short = 'a', long, help = "Path of audio output file")]
    pub audio_output: Option<String>,
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
