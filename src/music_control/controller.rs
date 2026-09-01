use num_enum::TryFromPrimitive;
use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};
use windows::{
    Foundation::TypedEventHandler,
    Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionMediaProperties,
    },
    Storage::Streams::DataReader,
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
        .map(|d| Duration::from_micros(d.saturating_div(10)))
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
pub struct ThumbnailInfo {
    pub extension: String,
    pub bytes: Arc<[u8]>,
}

impl ThumbnailInfo {
    fn load_blocking(
        props: &GlobalSystemMediaTransportControlsSessionMediaProperties,
    ) -> windows::core::Result<Self> {
        let stream = props.Thumbnail()?.OpenReadAsync()?.join()?;
        let size = u32::try_from(stream.Size()?)?;

        let input = stream.GetInputStreamAt(0)?;
        let reader = DataReader::CreateDataReader(&input)?;
        reader.LoadAsync(size)?.join()?;

        let mut bytes = vec![0u8; size as usize];
        reader.ReadBytes(&mut bytes)?;

        Ok(Self {
            extension: stream
                .ContentType()?
                .to_string()
                .split('/')
                .next_back()
                .unwrap()
                .to_owned(),
            bytes: bytes.into(),
        })
    }
}

#[derive(Debug)]
pub struct PlaybackInfo {
    pub track_name: Option<String>,
    pub artist: Option<String>,
    pub progress: Option<Duration>,
    pub song_length: Option<Duration>,
    pub status: Option<PlaybackStatus>,
    pub thumbnail: Option<ThumbnailInfo>,
}

impl PlaybackInfo {
    fn try_from_session(
        session: &GlobalSystemMediaTransportControlsSession,
    ) -> windows::core::Result<Self> {
        let props = session.TryGetMediaPropertiesAsync()?.join()?;
        let timeline = session.GetTimelineProperties().ok();

        Ok(Self {
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
            thumbnail: ThumbnailInfo::load_blocking(&props).ok(),
        })
    }

    fn create_listener_event_handler<A: windows::core::RuntimeType>(
        sender: mpsc::Sender<Option<PlaybackInfo>>,
    ) -> TypedEventHandler<GlobalSystemMediaTransportControlsSession, A> {
        TypedEventHandler::new(
            move |s: windows::core::Ref<GlobalSystemMediaTransportControlsSession>, _| {
                let Some(session) = s.as_ref() else {
                    return Ok(());
                };

                sender
                    .send(PlaybackInfo::try_from_session(session).ok())
                    .unwrap();

                Ok(())
            },
        )
    }

    fn listen_from_session(
        session: &GlobalSystemMediaTransportControlsSession,
        sender: &mpsc::Sender<Option<PlaybackInfo>>,
    ) {
        session
            .MediaPropertiesChanged(&Self::create_listener_event_handler(sender.clone()))
            .unwrap();

        session
            .TimelinePropertiesChanged(&Self::create_listener_event_handler(sender.clone()))
            .unwrap();
    }
}

pub struct MediaController {
    #[allow(unused)] // needs to stay alive or the CurrentSessionChanged listener will drop
    manager: GlobalSystemMediaTransportControlsSessionManager,
    current_session: Arc<Mutex<Option<GlobalSystemMediaTransportControlsSession>>>,
}

impl MediaController {
    pub fn new(playback_info_sender: mpsc::Sender<Option<PlaybackInfo>>) -> Self {
        let manager = request_manager().unwrap();

        let current_session = manager.GetCurrentSession().ok();
        if let Some(current_session) = &current_session {
            PlaybackInfo::listen_from_session(current_session, &playback_info_sender);
        } else {
            let _ = playback_info_sender.send(None);
        }

        let current_session = Arc::new(Mutex::new(current_session));
        let session_changer_ref = Arc::clone(&current_session);

        // rust cant infer TypedEventHandler types automatically for some reason
        manager
            .CurrentSessionChanged(&TypedEventHandler::<
                GlobalSystemMediaTransportControlsSessionManager,
                _,
            >::new(move |new_manager, _| {
                let new_session = new_manager.unwrap().GetCurrentSession().ok();
                let mut session = session_changer_ref.lock().unwrap();
                *session = new_session;

                if let Some(new_session) = session.as_ref() {
                    PlaybackInfo::listen_from_session(new_session, &playback_info_sender);
                } else {
                    let _ = playback_info_sender.send(None);
                }

                Ok(())
            }))
            .unwrap();

        Self {
            manager,
            current_session,
        }
    }

    pub fn next(&self) {
        self.use_session_once(GlobalSystemMediaTransportControlsSession::TrySkipNextAsync);
    }
    pub fn previous(&self) {
        self.use_session_once(GlobalSystemMediaTransportControlsSession::TrySkipPreviousAsync);
    }
    pub fn play(&self) {
        self.use_session_once(GlobalSystemMediaTransportControlsSession::TryPlayAsync);
    }
    pub fn pause(&self) {
        self.use_session_once(GlobalSystemMediaTransportControlsSession::TryPauseAsync);
    }

    fn use_session_once<F>(&self, func: F)
    where
        F: FnOnce(
                &GlobalSystemMediaTransportControlsSession,
            ) -> windows::core::Result<windows_future::IAsyncOperation<bool>>
            + Send
            + Sync
            + 'static,
    {
        self.use_session(|s| func(s)?.join().map(|_| ()));
    }

    fn use_session<F>(&self, func: F)
    where
        F: FnOnce(&GlobalSystemMediaTransportControlsSession) -> windows::core::Result<()>
            + Send
            + Sync
            + 'static,
    {
        let session = Arc::clone(&self.current_session);
        thread::spawn(move || {
            let session_guard = session.lock().unwrap();
            let Some(session) = session_guard.as_ref() else {
                return;
            };

            if let Err(error) = func(session) {
                eprintln!("winrt failure: {error:?}");
            }
        });
    }
}
