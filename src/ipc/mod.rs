use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod stream;
#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Ping,
    Play(Option<PathBuf>),
    Pause,
    Next,
    Previous,
    Append(PathBuf),
    Seek(f64),
    SetVolume(f64),
    Quit,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Pong,
    Success,
    Error(String),
}

pub fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("piri.sock")
    } else {
        PathBuf::from("/tmp/piri.sock")
    }
}
