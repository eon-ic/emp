use anyhow::Result;
use std::{net::TcpListener, sync::Arc};

use crate::ipc::{get_addr, stream::Stream};
#[derive(Debug)]
pub struct Daemon {
    server: Arc<TcpListener>,
}

impl Daemon {
    pub fn new() -> Result<Self> {
        let server = Arc::new(TcpListener::bind(get_addr())?);
        Ok(Self { server })
    }
    pub fn attach(&self, f: impl Fn(Stream) + Send + 'static + Sync) {
        let f = Arc::new(f);
        for _ in 0..1 {
            let server = self.server.clone();
            let f = f.clone();
            std::thread::spawn(move || {
                for stream in server.incoming() {
                    if let Ok(stream) = stream {
                        let stream = Stream::new(stream);
                        f(stream);
                    }
                }
            });
        }
    }
}
