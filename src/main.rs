use anyhow::Result;
use clap::Parser;
use std::{env, path::PathBuf};
mod cli;
mod core;
mod daemon;
mod ipc;
mod manager;

use crate::{
    cli::{Cli, Commands},
    core::Core,
    ipc::{Request, Response, stream::Stream},
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Daemon => {
            let mut core = Core::new()?;
            core.run().await?;
        }
        c => {
            let mut stream = Stream::get_client().await?;
            match c {
                Commands::Play { path } => {
                    if let Some(filename) = path {
                        let path = env::current_dir()
                            .unwrap_or_else(|_| PathBuf::from("."))
                            .join(filename);
                        stream.write(Request::Play(Some(path))).await?;
                    } else {
                        stream.write(Request::Play(None)).await?;
                    }
                }
                Commands::Ping => stream.write(Request::Ping).await?,
                Commands::Pause => stream.write(Request::Pause).await?,
                Commands::Next => stream.write(Request::Next).await?,
                Commands::Previous => stream.write(Request::Previous).await?,
                Commands::Seek { durtion } => stream.write(Request::Seek(durtion)).await?,
                Commands::Volume { volume } => stream.write(Request::SetVolume(volume)).await?,
                Commands::Quit => stream.write(Request::Quit).await?,
                _ => {}
            }
            match stream.read().await? {
                Response::Pong => println!("Pong"),
                Response::Success => println!("Success"),
                Response::Error(e) => eprintln!("{e}"),
                _ => {}
            }
        }
    };
    Ok(())
}
