use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::{
    common::{
        ReadWriteStateHandle, ThreadedReadWriteStateHandle, ThreadedReadonlyStateHandle,
        channel::{Receiver, Sender},
        eventsource::EventReceiver,
    },
    music_control::controller::{MediaController, PlaybackInfo},
    stats_api::RLEvent,
};

const UPDATE_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub enum MusicControlCommand {
    Next,
    Previous,
    Pause,
    Play,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MusicControlSettings {
    pub pause_for_anthems: bool,
}

#[derive(Debug, Default)]
pub struct MusicControlServiceState {
    pub playback_info: Option<PlaybackInfo>,
}

pub struct MusicControlService {
    controller: MediaController,
    settings: ReadWriteStateHandle<MusicControlSettings>,
    state: ThreadedReadWriteStateHandle<MusicControlServiceState>,
    command_receiver: Receiver<MusicControlCommand>,
    stats_api: EventReceiver<RLEvent>,
    last_update: SystemTime,
}

impl MusicControlService {
    pub fn new(savedata: Option<MusicControlSettings>, stats_api: EventReceiver<RLEvent>) -> Self {
        Self {
            controller: MediaController::new(),
            settings: ReadWriteStateHandle::new(savedata.unwrap_or_default()),
            state: ThreadedReadWriteStateHandle::default(),
            command_receiver: Receiver::new(),
            stats_api,
            last_update: SystemTime::UNIX_EPOCH,
        }
    }

    pub fn update(&mut self) {
        while let Some(cmd) = self.command_receiver.try_recv() {
            self.handle_command(cmd);
        }

        if self.settings.read().pause_for_anthems {
            while let Some(event) = self.stats_api.try_recv() {
                match *event {
                    RLEvent::ReplayStart | RLEvent::MatchOver(_) => {
                        self.handle_command(MusicControlCommand::Pause);
                    }
                    RLEvent::ReplayDone | RLEvent::MatchLeft => {
                        self.handle_command(MusicControlCommand::Play);
                    }
                    _ => {}
                }
            }
        } else {
            self.stats_api.drain();
        }

        let now = SystemTime::now();
        if now
            .duration_since(self.last_update)
            .is_ok_and(|d| d >= UPDATE_INTERVAL)
        {
            self.last_update = now;

            let state = self.state.clone();
            self.controller.get_playback_info(move |playback_info| {
                state.write().playback_info = playback_info;
            });
        }
    }

    fn handle_command(&mut self, command: MusicControlCommand) {
        match command {
            MusicControlCommand::Next => self.controller.next(),
            MusicControlCommand::Previous => self.controller.previous(),
            MusicControlCommand::Play => self.controller.play(),
            MusicControlCommand::Pause => self.controller.pause(),
        };
    }

    pub fn sender(&self) -> Sender<MusicControlCommand> {
        self.command_receiver.send()
    }

    pub fn state_handle(&self) -> ThreadedReadonlyStateHandle<MusicControlServiceState> {
        ThreadedReadonlyStateHandle::over(&self.state)
    }

    pub fn settings_handle(&self) -> ReadWriteStateHandle<MusicControlSettings> {
        ReadWriteStateHandle::clone(&self.settings)
    }
}
