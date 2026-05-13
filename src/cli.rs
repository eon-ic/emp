#[derive(clap::Parser)]
#[command(name = "emp", about = "一个音乐播放器后台服务")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
#[derive(clap::Subcommand, Clone)]
pub enum Commands {
    Daemon,
    Ping,
    Play { path: Option<String> },
    Pause,
    Next,
    Previous,
    Seek { durtion: f64 },
    Volume { volume: f64 },
    Quit,
}
