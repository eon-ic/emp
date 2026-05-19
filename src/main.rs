use anyhow::Result;
use clap::Parser;
use std::{env, path::PathBuf};
mod cli;
mod core;
mod daemon;
mod ipc;
mod manager;
mod stopwatch;

use crate::{
    cli::{Cli, Commands},
    core::Core,
    ipc::{Request, Response, stream::Stream},
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Daemon => {
            let mut core = Core::new()?;
            core.run()?;
        }
        c => {
            let mut stream = Stream::get_client()?;
            match c {
                Commands::Play { path } => {
                    if let Some(filename) = path {
                        let path = env::current_dir()
                            .unwrap_or_else(|_| PathBuf::from("."))
                            .join(filename);
                        stream.write(Request::Play(Some(path)))?;
                    } else {
                        stream.write(Request::Play(None))?;
                    }
                }
                Commands::Ping => stream.write(Request::Ping)?,
                Commands::Pause => stream.write(Request::Pause)?,
                Commands::Next => stream.write(Request::Next)?,
                Commands::Previous => stream.write(Request::Previous)?,
                Commands::Seek { durtion } => stream.write(Request::Seek(durtion))?,
                Commands::Volume { volume } => stream.write(Request::SetVolume(volume))?,
                Commands::Quit => stream.write(Request::Quit)?,
                _ => {}
            }
            match stream.read()? {
                Response::Pong => println!("Pong"),
                Response::Success => println!("Success"),
                Response::Error(e) => eprintln!("{e}"),
                _ => {}
            }
        }
    };
    Ok(())
}
