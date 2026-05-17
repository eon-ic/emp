use crate::core::Core;
use anyhow::Result;
mod cli;
mod core;
mod daemon;
mod ipc;
mod manager;
pub struct LibEmp {
    core: Core,
}

impl LibEmp {
    pub fn new() -> Result<Self> {
        Ok(Self { core: Core::new()? })
    }
    pub fn run(mut self) -> Result<()> {
        std::thread::spawn(move || self.core.run());
        Ok(())
    }
}
