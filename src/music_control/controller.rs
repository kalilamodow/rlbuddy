use num_enum::TryFromPrimitive;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
};

fn request_manager() -> Option<GlobalSystemMediaTransportControlsSessionManager> {
    GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .ok()
        .and_then(|a| a.join().ok())
}

fn timespan_to_duration(
    timespan: windows::core::Result<windows::Foundation::TimeSpan>,
) -> Option<Duration> {
    timespan
        .ok()
        .and_then(|p| u64::try_from(p.Duration).ok())
        .map(|d| Duration::from_micros(d))
}

#[derive(Debug, TryFromPrimitive)]
#[repr(i32)]
pub enum PlaybackStatus {
    Closed = 0,
    Opened = 1,
    Changing = 2,
    Stopped = 3,
    Playing = 4,
    Paused = 5,
}

#[derive(Debug)]
pub struct PlaybackInfo {
    pub track_name: Option<String>,
    pub artist: Option<String>,
    pub progress: Option<Duration>,
    pub song_length: Option<Duration>,
    pub status: Option<PlaybackStatus>,
}

pub struct MediaController {
    manager: Arc<Mutex<Option<GlobalSystemMediaTransportControlsSessionManager>>>,
}

impl MediaController {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(Mutex::new(request_manager())),
        }
    }

    pub fn get_playback_info<F>(&self, callback: F)
    where
        F: FnOnce(Option<PlaybackInfo>) + Send + Sync + 'static,
    {
        self.start(|manager| {
            let Ok(session) = manager.GetCurrentSession() else {
                callback(None);
                return Ok(());
            };

            let props = session.TryGetMediaPropertiesAsync()?.join()?;
            let timeline = session.GetTimelineProperties().ok();

            callback(Some(PlaybackInfo {
                track_name: props.Title().map(|t| t.to_string()).ok(),
                artist: props.Artist().map(|t| t.to_string()).ok(),
                progress: timeline
                    .as_ref()
                    .and_then(|time| timespan_to_duration(time.Position())),
                song_length: timeline
                    .as_ref()
                    .and_then(|time| timespan_to_duration(time.EndTime())),
                status: PlaybackStatus::try_from_primitive(
                    session.GetPlaybackInfo()?.PlaybackStatus()?.0,
                )
                .ok(),
            }));

            Ok(())
        });
    }

    pub fn next(&self) {
        self.use_session(GlobalSystemMediaTransportControlsSession::TrySkipNextAsync);
    }
    pub fn previous(&self) {
        self.use_session(GlobalSystemMediaTransportControlsSession::TrySkipPreviousAsync);
    }
    pub fn play(&self) {
        self.use_session(GlobalSystemMediaTransportControlsSession::TryPlayAsync);
    }
    pub fn pause(&self) {
        self.use_session(GlobalSystemMediaTransportControlsSession::TryPauseAsync);
    }

    fn use_session<F>(&self, func: F)
    where
        F: FnOnce(
                &GlobalSystemMediaTransportControlsSession,
            ) -> windows::core::Result<windows_future::IAsyncOperation<bool>>
            + Send
            + Sync
            + 'static,
    {
        self.start(|manager| {
            let session = manager.GetCurrentSession()?;
            func(&session)?.join()?;
            Ok(())
        });
    }

    fn start<F>(&self, func: F)
    where
        F: FnOnce(&GlobalSystemMediaTransportControlsSessionManager) -> windows::core::Result<()>
            + Send
            + Sync
            + 'static,
    {
        let manager = Arc::clone(&self.manager);
        thread::spawn(move || -> windows::core::Result<()> {
            let mut manager = manager.lock().unwrap();
            if manager.is_none() {
                *manager = request_manager();
            };

            if let Some(manager) = &*manager {
                func(manager)
            } else {
                Err(windows::core::Error::empty())
            }
        });
    }
}
