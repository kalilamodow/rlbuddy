use crate::common::savedata::load_service_data;
use crate::core::app::{Panel, Service, ServiceWithUi};
use crate::music_control::widget::MusicControlWidget;
use crate::stats_api::StatsApi;
use crate::{
    common::{
        ReadWriteStateHandle, ThreadedReadWriteStateHandle, ThreadedReadonlyStateHandle,
        channel::{Receiver, Sender},
        eventsource::EventReceiver,
    },
    music_control::controller::{MediaController, PlaybackInfo},
    stats_api::RLEvent,
};
use serde::{Deserialize, Serialize};
use std::sync::mpsc;

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
    playback_info_rx: mpsc::Receiver<Option<PlaybackInfo>>,
}

const DATA_ID: &str = "music_control_settings";

impl MusicControlService {
    pub fn new(stats_api: &mut StatsApi) -> Self {
        let (playback_info_tx, playback_info_rx) = mpsc::channel();

        Self {
            controller: MediaController::new(playback_info_tx),
            settings: ReadWriteStateHandle::new(load_service_data(DATA_ID)),
            state: ThreadedReadWriteStateHandle::default(),
            command_receiver: Receiver::new(),
            stats_api: stats_api.subscribe(),
            playback_info_rx,
        }
    }

    pub fn update(&mut self) {
        while let Some(cmd) = self.command_receiver.try_recv() {
            self.handle_command(&cmd);
        }

        for new_info in self.playback_info_rx.try_iter() {
            self.state.write().playback_info = new_info;
        }

        if self.settings.read().pause_for_anthems {
            while let Some(event) = self.stats_api.try_recv() {
                match *event {
                    RLEvent::ReplayStart | RLEvent::MatchOver(_) => {
                        self.handle_command(&MusicControlCommand::Pause);
                    }
                    RLEvent::ReplayDone | RLEvent::MatchLeft => {
                        self.handle_command(&MusicControlCommand::Play);
                    }
                    _ => {}
                }
            }
        } else {
            self.stats_api.drain();
        }
    }

    fn handle_command(&mut self, command: &MusicControlCommand) {
        match command {
            MusicControlCommand::Next => self.controller.next(),
            MusicControlCommand::Previous => self.controller.previous(),
            MusicControlCommand::Play => self.controller.play(),
            MusicControlCommand::Pause => self.controller.pause(),
        }
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

impl Service for MusicControlService {
    fn update(&mut self) {
        self.update();
    }
}

impl ServiceWithUi for MusicControlService {
    fn panel(&self) -> impl Panel + 'static {
        MusicControlWidget::new(self)
    }
}
