use std::{fs::File, io::BufReader, path::PathBuf, sync::Arc, time::Duration};

use anyhow::Result;
use lofty::{file::TaggedFileExt, picture::MimeType, tag::Accessor};
use rodio::{DeviceSinkBuilder, Player, Source, decoder::DecoderBuilder};
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPosition, PlatformConfig};

use crate::{
    daemon::Daemon,
    ipc::{Request, Response, stream::Stream},
    manager::MusicManager,
};

pub struct Core {
    _mixer_device_sink: rodio::MixerDeviceSink,
    pub(crate) player: Arc<Player>,
    pub(crate) playing_time: Arc<std::sync::Mutex<f64>>,
    music_manager: MusicManager,
    controls: Box<MediaControls>,
    daemon: Daemon,
    total_duration: Option<Duration>,
    is_playing: bool,
    running: bool,
}

impl Core {
    pub fn new() -> Result<Self> {
        let playing_time = Arc::new(std::sync::Mutex::new(0.0));

        let _mixer_device_sink = DeviceSinkBuilder::open_default_sink()
            .expect("无法打开默认音频输出设备。请确保你的系统有可用的音频设备");
        let player = Arc::new(rodio::Player::connect_new(_mixer_device_sink.mixer()));
        let music_manager = MusicManager::new();
        #[cfg(not(target_os = "windows"))]
        let hwnd = None;
        #[cfg(target_os = "windows")]
        let hwnd = Some(souvlaki::platform::windows::get_hwnd().expect("无法获取窗口句柄"));
        let config = PlatformConfig {
            dbus_name: "com.eon.bmusic",
            display_name: "BMusic",
            hwnd,
        };

        let controls =
            MediaControls::new(config).expect("无法初始化媒体控制。请确保你的系统支持媒体控制");
        Ok(Self {
            _mixer_device_sink,
            player,
            playing_time,
            controls: Box::new(controls),
            music_manager,
            daemon: Daemon::new()?,
            total_duration: None,
            is_playing: false,
            running: true,
        })
    }
    pub fn pause(&mut self) -> Result<Response> {
        self.is_playing = false;
        self.player.pause();
        Ok(Response::Success)
    }
    pub fn next(&mut self) -> Result<Response> {
        *self.playing_time.lock().unwrap() = 0.0;
        self.music_manager.next();
        self.play()
    }
    pub fn previous(&mut self) -> Result<Response> {
        *self.playing_time.lock().unwrap() = 0.0;
        self.music_manager.previous();
        self.play()
    }
    pub fn seek(&mut self, duration: f64) -> Result<Response> {
        // 1. 预检查：如果 duration 是负数或 NaN，提前拦截
        if duration < 0.0 || duration.is_nan() {
            return Ok(Response::Error("无效的跳转时间".into()));
        }
        *self.playing_time.clone().lock().unwrap() = duration;
        self.play()
    }
    pub fn set_volume(&mut self, volume: f64) -> Result<Response> {
        self.player.set_volume(volume as f32);
        Ok(Response::Success)
    }
    pub fn play(&mut self) -> Result<Response> {
        self.is_playing = true;
        let is_backend = *self.playing_time.lock().unwrap() != 0.0;
        if let Ok(path) = self.music_manager.get() {
            let file = File::open(&path)?;
            let tagged_file = lofty::probe::read_from_path(&path)?;
            let buffer_read = BufReader::new(file);
            let mut source = DecoderBuilder::new().with_data(buffer_read).build()?;
            if is_backend {
                self.player.pause();
                self.player.clear();
                source.try_seek(Duration::from_secs_f64(*self.playing_time.lock().unwrap()))?;
                self.player.append(source);
                self.player.play();
            } else {
                let duration = source.total_duration();
                self.total_duration = duration;
                if let Some(tag) = tagged_file.primary_tag() {
                    let mut cover_url = None;
                    let pictures = tag.pictures();
                    if !pictures.is_empty() {
                        // 这里我们简单地将封面图片保存到临时文件，并使用 file:// URI
                        let picture = &pictures[0];
                        let temp_path = std::env::temp_dir().join({
                            let s = String::from("bmusic-cover.");
                            let t = match picture.mime_type().unwrap_or(&MimeType::Png) {
                                MimeType::Jpeg => "jpg",
                                MimeType::Png => "png",
                                _ => "png",
                            };
                            s + t
                        });
                        std::fs::write(&temp_path, picture.data())?;
                        cover_url = Some(format!("file://{}", temp_path.to_string_lossy()));
                    }
                    // 甚至可以获取封面controls
                    self.controls.set_metadata(MediaMetadata {
                        title: tag.title().as_deref(),
                        artist: tag.artist().as_deref(),
                        album: tag.album().as_deref(),
                        duration: duration, // 如果有可以用 Duration 传入
                        cover_url: cover_url.as_deref(), // 可以是本地文件的 file:// URI
                    })?;
                };
                self.player.clear();
                self.player.append(source);
                self.player.play();
            }
            Ok(Response::Success)
        } else {
            Ok(Response::Error("没有音乐".to_string()))
        }
    }
    pub fn run(&mut self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel::<MediaControlEvent>();
        self.controls.attach(move |event| tx.send(event).unwrap())?;
        loop {
            if let Ok(event) = rx.recv_timeout(Duration::from_millis(100)) {
                self.media_event_handle(event)?;
            }
            if let Ok(stream) = self.daemon.accept() {
                self.daemon_event_handle(stream)?;
            }
            if self.is_playing {
                *self.playing_time.lock().unwrap() += 0.1;
                self.controls
                    .set_playback(souvlaki::MediaPlayback::Playing {
                        progress: Some(MediaPosition(Duration::from_secs_f64(
                            *self.playing_time.lock().unwrap(),
                        ))),
                    })?;
            } else {
                self.controls
                    .set_playback(souvlaki::MediaPlayback::Paused {
                        progress: Some(MediaPosition(Duration::from_secs_f64(
                            *self.playing_time.lock().unwrap(),
                        ))),
                    })?;
            }
            if let Some(duration) = self.total_duration {
                if duration.as_secs_f64() <= *self.playing_time.lock().unwrap() {
                    self.pause()?;
                    self.next()?;
                }
            }
            if !self.running {
                break;
            }
        }
        Ok(())
    }
    pub fn append(&mut self, path: PathBuf) -> Result<Response> {
        self.music_manager.append(path);
        Ok(Response::Success)
    }
    pub fn append_and_play(&mut self, path: PathBuf) -> Result<Response> {
        *self.playing_time.lock().unwrap() = 0.0;
        self.music_manager.append_and_next(path);
        self.play()
    }
    pub fn daemon_event_handle(&mut self, mut stream: Stream) -> Result<()> {
        let result: Request = stream.read()?;
        let req = match result {
            Request::Ping => Ok(crate::ipc::Response::Pong),
            Request::Append(path) => self.append(path),
            Request::Play(path) => match path {
                Some(path) => self.append_and_play(path),
                None => self.play(),
            },
            Request::Pause => self.pause(),
            Request::Next => self.next(),
            Request::Previous => self.previous(),
            Request::Seek(duration) => self.seek(duration),
            Request::SetVolume(volume) => self.set_volume(volume),
            Request::Quit => {
                self.running = false;
                Ok(Response::Success)
            }
        }?;
        stream.write(req)?;
        Ok(())
    }
    pub fn media_event_handle(&mut self, event: MediaControlEvent) -> Result<Response> {
        match event {
            MediaControlEvent::Play => self.play(),
            MediaControlEvent::Pause => self.pause(),
            MediaControlEvent::Previous => self.previous(),
            MediaControlEvent::Next => self.next(),
            MediaControlEvent::SeekBy(seek, _) | MediaControlEvent::Seek(seek) => match seek {
                souvlaki::SeekDirection::Backward => self.seek(-5.0),
                souvlaki::SeekDirection::Forward => self.seek(5.0),
            },
            MediaControlEvent::SetPosition(pos) => self.seek(pos.0.as_secs_f64()),
            MediaControlEvent::Toggle => {
                if self.player.is_paused() {
                    self.play()
                } else {
                    self.pause()
                }
            }
            MediaControlEvent::SetVolume(volume) => self.set_volume(volume),
            e => Ok(Response::Error(format!("Not Set {:?}", e))),
        }
        // 播放歌曲
    }
}
