use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    io::{Read, Write},
    net::TcpStream,
};

use crate::ipc::get_addr;

pub struct Stream(TcpStream);

impl Stream {
    #[allow(unused)]
    pub fn get_client() -> Result<Self> {
        let client = TcpStream::connect(get_addr())?;
        let stream = Stream::new(client);
        Ok(stream)
    }
    pub fn new(stream: TcpStream) -> Self {
        Self(stream)
    }
    pub fn write<T>(&mut self, value: T) -> Result<()>
    where
        T: Serialize,
    {
        let buf = postcard::to_allocvec(&value)?;
        self.0.write(&buf.len().to_le_bytes())?;
        self.0.write_all(&mut buf.as_slice())?;
        Ok(())
    }

    pub fn read<T>(&mut self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut len = [0u8; 8];
        self.0.read_exact(&mut len)?;
        let mut buf = vec![0u8; usize::from_le_bytes(len)];
        self.0.read_exact(&mut buf)?;
        Ok(postcard::from_bytes::<T>(&buf).expect("解析数据失败"))
    }
}
