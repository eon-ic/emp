use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
};

use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};

use crate::ipc::get_socket_path;

pub struct Stream(UnixStream);

impl Stream {
    pub fn get_client() -> Result<Self> {
        let client = UnixStream::connect(get_socket_path())?;
        let stream = Stream::new(client);
        Ok(stream)
    }
    pub fn new(stream: UnixStream) -> Self {
        Self(stream)
    }
    pub fn write<T>(&mut self, value: T) -> Result<()>
    where
        T: Serialize,
    {
        let buf = &postcard::to_vec::<T, 128>(&value).expect("写数据失败");
        self.0.write(&(buf.len() as u32).to_le_bytes())?;
        self.0.write_all(buf)?;
        Ok(())
    }
    pub fn read<T>(&mut self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut len = [0u8; 4];
        self.0.read(&mut len)?;
        let len = u32::from_le_bytes(len) as usize;
        let mut buffer = vec![0u8; len];
        self.0.read_exact(&mut buffer)?;

        Ok(postcard::from_bytes::<T>(&buffer).expect("解析数据失败"))
    }
}
