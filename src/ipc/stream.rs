use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

use crate::ipc::get_socket_path;

pub struct Stream(UnixStream);

impl Stream {
    #[allow(unused)]
    pub async fn get_client() -> Result<Self> {
        let client = UnixStream::connect(get_socket_path()).await?;
        let stream = Stream::new(client);
        Ok(stream)
    }
    pub fn new(stream: UnixStream) -> Self {
        Self(stream)
    }
    pub async fn write<T>(&mut self, value: T) -> Result<()>
    where
        T: Serialize,
    {
        let buf = &postcard::to_vec::<T, 64>(&value).expect("写数据失败");
        self.0.write_u32(buf.len() as u32).await?;
        self.0.write_all(buf).await?;
        Ok(())
    }
    pub async fn read<T>(&mut self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let len = self.0.read_u32().await?;
        let mut buf = vec![0u8; len as usize];
        self.0.read_exact(&mut buf).await?;
        Ok(postcard::from_bytes::<T>(&buf).expect("解析数据失败"))
    }
}
