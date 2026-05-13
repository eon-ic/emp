use std::{fs, os::unix::net::UnixListener};

use anyhow::Result;

use crate::ipc::{get_socket_path, stream::Stream};

pub struct Daemon {
    server: UnixListener,
}

impl Daemon {
    pub fn new() -> Result<Self> {
        let path = get_socket_path();
        if fs::metadata(&path).is_ok() {
            fs::remove_file(&path)?;
        }
        let server = UnixListener::bind(&path)?;
        Ok(Self { server })
    }
    pub fn accept(&mut self) -> Result<Stream> {
        self.server.set_nonblocking(true).expect("无法设置为非阻塞");
        let (stream, _) = self.server.accept()?;
        Ok(Stream::new(stream))
    }
}
