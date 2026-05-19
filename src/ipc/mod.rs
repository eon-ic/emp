use std::path::PathBuf;

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayStatus {
    pub playing: bool,
    pub position: u64,
    pub duration: u64,
    pub cover: Option<PathBuf>,
    pub music_name: String,
    pub artist: String,
}

pub mod stream;
#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Ping,
    Pause,
    Play(Option<PathBuf>),
    Next,
    Previous,
    Append(PathBuf),
    Seek(f64),
    SetVolume(f64),
    Status,
    Quit,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Pong,
    Success,
    Error(String),
    Status(PlayStatus),
}
pub fn get_addr() -> &'static str {
    "127.0.0.1:6600"
}
// pub fn get_socket_path() -> PathBuf {
//     if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
//         PathBuf::from(runtime_dir).join("piri.sock")
//     } else {
//         PathBuf::from("/tmp/piri.sock")
//     }
// }
