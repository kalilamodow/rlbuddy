// SO. to save replays:
// 1. use the save replay hotkey at the start of the match to make it save whenever the match ends
// 2. once the match ends, it automatically prompts to save after 8.5 seconds (podium showing)!! so accept that ASAP
// 3. then it gets saved to a file, use it however :)

use crate::{
    common::{
        ReadWriteStateHandle,
        channel::Receiver,
        savedata::{load_service_config, save_service_config},
    },
    core::app::{Service, ServiceWithUi},
    replay_saver::panel::ReplaySaverWidget,
    stats_api::{RLEvent, StatsApi},
};
use rdev::{EventType, Key};
use serde::{Deserialize, Serialize};
use std::{rc::Rc, thread, time::Duration};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChoosableKey {
    Alt,
    Shift,
    Ctrl,
    Backtick,
    Backslash,
    Insert,
    Enter,
}

impl ChoosableKey {
    pub const ALL: [ChoosableKey; 7] = [
        ChoosableKey::Alt,
        ChoosableKey::Shift,
        ChoosableKey::Ctrl,
        ChoosableKey::Backtick,
        ChoosableKey::Backslash,
        ChoosableKey::Insert,
        ChoosableKey::Enter,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ChoosableKey::Alt => "alt",
            ChoosableKey::Shift => "shift",
            ChoosableKey::Ctrl => "ctrl",
            ChoosableKey::Backtick => "backtick",
            ChoosableKey::Backslash => "backslash",
            ChoosableKey::Insert => "insert",
            ChoosableKey::Enter => "enter",
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct ReplaySaverSettings {
    pub chosen_key: Option<ChoosableKey>,
    pub save_non_comp: bool,
}

pub type ReplaySaverSettingsHandle = ReadWriteStateHandle<ReplaySaverSettings>;

pub struct ReplaySaverService {
    stats_api: Receiver<Rc<RLEvent>>,
    settings: ReplaySaverSettingsHandle,
    next_match_end_should_save: bool,
}

static DATA_ID: &str = "replay-saver";

impl ReplaySaverService {
    pub fn new(stats_api: &mut StatsApi) -> Self {
        Self {
            stats_api: stats_api.subscribe(),
            settings: ReadWriteStateHandle::new(load_service_config(DATA_ID)),
            next_match_end_should_save: false,
        }
    }

    pub fn settings_handle(&self) -> ReplaySaverSettingsHandle {
        self.settings.clone()
    }
}

impl Service for ReplaySaverService {
    fn update(&mut self) {
        let settings = self.settings.read();
        if settings.chosen_key.is_none() {
            // disabled
            self.stats_api.drain();
            return;
        }

        while let Some(event) = self.stats_api.try_recv() {
            match &*event {
                RLEvent::MatchStart(data)
                    if data.is_online && (settings.save_non_comp || data.playlist.is_comp()) =>
                {
                    self.next_match_end_should_save = true;
                    thread::spawn(|| {
                        if rdev::simulate(&EventType::KeyPress(Key::BackSlash)).is_err() {
                            return;
                        }

                        thread::sleep(Duration::from_millis(1500));
                        let _ = rdev::simulate(&EventType::KeyRelease(Key::BackSlash));
                    });
                }
                RLEvent::MatchOver(_) if self.next_match_end_should_save => {
                    self.next_match_end_should_save = false;
                    thread::spawn(|| {
                        thread::sleep(Duration::from_millis(8_500));
                        let _ = rdev::simulate(&EventType::KeyPress(Key::Return));
                        thread::sleep(Duration::from_millis(100));
                        let _ = rdev::simulate(&EventType::KeyRelease(Key::Return));
                    });
                }
                _ => {}
            }
        }
    }

    fn save(&self) {
        save_service_config(DATA_ID, &*self.settings.read());
    }
}

impl ServiceWithUi for ReplaySaverService {
    fn panel(&self) -> impl crate::core::app::Panel + 'static {
        ReplaySaverWidget::new(self)
    }
}
