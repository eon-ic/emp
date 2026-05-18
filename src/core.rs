use std::{fs::File, io::BufReader, path::PathBuf, sync::Arc, time::Duration};

use anyhow::Result;
use lofty::{file::TaggedFileExt, picture::MimeType, tag::Accessor};
use rodio::{DeviceSinkBuilder, Player, Source, decoder::DecoderBuilder};
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPosition, PlatformConfig};
use tokio::{runtime::Runtime, sync::Mutex, time::timeout};

use crate::{
    daemon::Daemon,
    ipc::{PlayStatus, Request, Response, stream::Stream},
    manager::MusicManager,
};

pub struct Core {
    _mixer_device_sink: rodio::MixerDeviceSink,
    pub(crate) player: Arc<Player>,
    pub(crate) playing_time: Arc<Mutex<f64>>,
    play_status: Option<PlayStatus>,
    music_manager: MusicManager,
    controls: Box<MediaControls>,
    daemon: Daemon,
    total_duration: Option<Duration>,
    is_playing: bool,
    is_seek: bool,
    running: bool,
}

impl Core {
    pub fn new() -> Result<Self> {
        let playing_time = Arc::new(Mutex::new(0.0));
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
            play_status: None,
            is_seek: false,
        })
    }
    pub async fn pause(&mut self) -> Result<Response> {
        self.is_playing = false;
        self.player.pause();
        self.controls
            .set_playback(souvlaki::MediaPlayback::Paused {
                progress: Some(MediaPosition(Duration::from_secs_f64(
                    *self.playing_time.lock().await,
                ))),
            })?;
        Ok(Response::Success)
    }
    pub async fn next(&mut self) -> Result<Response> {
        *self.playing_time.lock().await = 0.0;
        self.music_manager.next();
        self.play().await
    }
    pub async fn previous(&mut self) -> Result<Response> {
        *self.playing_time.lock().await = 0.0;
        self.music_manager.previous();
        self.play().await
    }
    pub async fn seek(&mut self, duration: f64) -> Result<Response> {
        if duration < 0.0 || duration.is_nan() {
            return Ok(Response::Error("无效的跳转时间".into()));
        }
        *self.playing_time.lock().await = duration;
        self.is_seek = true;
        self.play().await
    }
    pub fn set_volume(&mut self, volume: f64) -> Result<Response> {
        self.player.set_volume(volume as f32);
        Ok(Response::Success)
    }
    pub async fn play(&mut self) -> Result<Response> {
        let play_time = *self.playing_time.lock().await;
        if let Ok(path) = self.music_manager.get_path() {
            self.controls
                .set_playback(souvlaki::MediaPlayback::Playing {
                    progress: Some(MediaPosition(Duration::from_secs_f64(play_time))),
                })?;
            self.is_playing = true;
            let file = File::open(&path)?;
            let bufread = BufReader::new(file.try_clone()?);
            let mut source = DecoderBuilder::new().with_data(bufread).build()?;
            if self.is_seek {
                source.try_seek(Duration::from_secs_f64(*self.playing_time.lock().await))?;
                self.player.pause();
                self.player.clear();
                self.player.append(source);
                self.player.play();
                self.is_seek = false
            } else if play_time != 0.0 {
                self.player.play();
            } else {
                let tagged_file = lofty::probe::read_from_path(&path)?;
                let duration = source.total_duration();
                self.total_duration = duration;
                if let Some(tag) = tagged_file.primary_tag() {
                    let mut cover_url = None;
                    let mut temp = None;
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
                        temp = Some(temp_path.clone());
                        cover_url = Some(format!("file://{}", temp_path.to_string_lossy()));
                    }
                    self.play_status = Some(PlayStatus {
                        playing: self.is_playing,
                        position: 0,
                        duration: duration.and_then(|d| Some(d.as_secs())).unwrap_or(0),
                        cover: temp,
                        music_name: tag.title().as_deref().unwrap_or("Unknown").to_string(),
                        artist: tag
                            .artist()
                            .as_deref()
                            .unwrap_or("Unknown Artist")
                            .to_string(),
                    });
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
    pub async fn run(&mut self) -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<MediaControlEvent>(32);
        self.controls.attach(move |event| {
            let tx = tx.clone();
            Runtime::new().unwrap().spawn(async move {
                tx.send(event).await.unwrap();
            });
        })?;

        while self.running {
            tokio::select! {
                //  Some(event) =rx.recv()=> {
                //     self.media_event_handle(event).await?;
                // }
                // Ok(stream)= self.daemon.accept()=> {
                //     self.daemon_event_handle(stream).await?;
                // }
                Ok(Some(event)) =timeout(Duration::from_millis(25),rx.recv()) => {
                    self.media_event_handle(event).await?;
                }
                Ok(Ok(stream)) = timeout(Duration::from_millis(25),self.daemon.accept()) => {
                    self.daemon_event_handle(stream).await?;
                }
                else => {
                    let play_time = *self.playing_time.lock().await;

                    if self.is_playing {
                        *self.playing_time.lock().await = play_time + 0.025;
                    }
                    if let Some(duration) = self.total_duration {
                        if duration.as_secs_f64() <= *self.playing_time.lock().await {
                            self.pause().await?;
                            self.next().await?;
                        }
                    }}
            }
        }
        Ok(())
    }
    pub fn append(&mut self, path: PathBuf) -> Result<Response> {
        self.music_manager.append(path);
        Ok(Response::Success)
    }
    pub async fn append_and_play(&mut self, path: PathBuf) -> Result<Response> {
        *self.playing_time.lock().await = 0.0;
        self.music_manager.append_and_next(path);
        self.play().await
    }
    pub async fn send_status(&self) -> PlayStatus {
        let pos = *self.playing_time.lock().await;
        self.play_status
            .clone()
            .map(|mut d| {
                d.playing = self.is_playing;
                d.position = pos as u64;
                d
            })
            .unwrap_or(PlayStatus {
                playing: self.is_playing,
                position: (*self.playing_time.lock().await) as u64,
                duration: self
                    .total_duration
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs(),
                cover: None,
                music_name: "Unknow".to_string(),
                artist: "Unknow".to_string(),
            })
    }
    pub async fn daemon_event_handle(&mut self, mut stream: Stream) -> Result<()> {
        let result: Request = stream.read().await?;
        let req = match result {
            Request::Ping => Ok(crate::ipc::Response::Pong),
            Request::Append(path) => self.append(path),
            Request::Play(path) => match path {
                Some(path) => self.append_and_play(path).await,
                None => self.play().await,
            },
            Request::Pause => self.pause().await,
            Request::Next => self.next().await,
            Request::Previous => self.previous().await,
            Request::Seek(duration) => self.seek(duration).await,
            Request::SetVolume(volume) => self.set_volume(volume),
            Request::Status => Ok(Response::Status(self.send_status().await)),
            Request::Quit => {
                self.running = false;
                Ok(Response::Success)
            }
        }?;
        stream.write(req).await?;
        Ok(())
    }
    pub async fn media_event_handle(&mut self, event: MediaControlEvent) -> Result<Response> {
        match event {
            MediaControlEvent::Play => self.play().await,
            MediaControlEvent::Pause => self.pause().await,
            MediaControlEvent::Previous => self.previous().await,
            MediaControlEvent::Next => self.next().await,
            MediaControlEvent::SeekBy(seek, _) | MediaControlEvent::Seek(seek) => match seek {
                souvlaki::SeekDirection::Backward => self.seek(-5.0).await,
                souvlaki::SeekDirection::Forward => self.seek(5.0).await,
            },
            MediaControlEvent::SetPosition(pos) => self.seek(pos.0.as_secs_f64()).await,
            MediaControlEvent::Toggle => {
                if self.player.is_paused() {
                    self.play().await
                } else {
                    self.pause().await
                }
            }
            MediaControlEvent::SetVolume(volume) => self.set_volume(volume),
            e => Ok(Response::Error(format!("Not Set {:?}", e))),
        }
    }
}
