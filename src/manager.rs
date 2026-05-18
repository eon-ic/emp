use std::path::PathBuf;

use anyhow::{Context, Result};

pub struct MusicManager {
    musics: Vec<PathBuf>,
    index: i32,
}

impl MusicManager {
    pub fn new() -> Self {
        Self {
            musics: vec![],
            index: 0,
        }
    }
    pub fn append(&mut self, path: PathBuf) {
        self.musics.push(path);
    }
    pub fn append_and_next(&mut self, path: PathBuf) {
        self.next();
        self.musics.insert(self.index as usize, path);
    }
    pub fn get_path(&self) -> Result<PathBuf> {
        let a = self
            .musics
            .get(self.index as usize)
            .context("无法找到Music")?;
        Ok(a.clone())
    }
    pub fn next(&mut self) {
        if self.index + 1 >= self.musics.len() as i32 {
            self.index = 0
        } else {
            self.index += 1;
        }
    }
    pub fn previous(&mut self) {
        if self.index - 1 < 0 {
            let len = self.musics.len();
            self.index = if len <= 1 { 0 } else { len as i32 - 1 };
        } else {
            self.index -= 1
        }
    }
}
