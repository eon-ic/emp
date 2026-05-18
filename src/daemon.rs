use std::fs;

use anyhow::Result;
use tokio::net::UnixListener;

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
    pub async fn accept(&mut self) -> Result<Stream> {
        let (stream, _) = self.server.accept().await?;
        Ok(Stream::new(stream))
    }
}
