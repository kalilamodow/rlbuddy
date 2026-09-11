use crate::common::eventsource::EventReceiver;
use crate::common::savedata::{load_service_config, save_service_config};
use crate::common::{ThreadedReadWriteStateHandle, ThreadedReadonlyStateHandle};
use crate::core::app::{Panel, Service, ServiceWithUi};
use crate::gamepad::{GamepadEvent, GamepadService};
use crate::hotkey::HotkeySettingsWidget;
use gilrs::Button;
use rdev::Key;
use serde::{Deserialize, Serialize};
use std::{sync::mpsc, thread};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectableHotkey {
    #[default]
    Alt,
    LShift,
    LCtrl,
    Tab,
    Super,
    Disabled,
}

impl SelectableHotkey {
    pub fn to_rdev(&self) -> Option<Key> {
        Some(match self {
            SelectableHotkey::Disabled => return None,
            SelectableHotkey::Alt => Key::Alt,
            SelectableHotkey::LShift => Key::ShiftLeft,
            SelectableHotkey::LCtrl => Key::ControlLeft,
            SelectableHotkey::Tab => Key::Tab,
            SelectableHotkey::Super => Key::MetaLeft,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SelectableHotkey::Disabled => "Disabled",
            SelectableHotkey::Alt => "Alt",
            SelectableHotkey::LShift => "Left Shift",
            SelectableHotkey::LCtrl => "Left Ctrl",
            SelectableHotkey::Tab => "Tab",
            SelectableHotkey::Super => "Windows",
        }
    }
}

struct KeyboardInputManager {
    tx: mpsc::Sender<bool>,
    settings: ThreadedReadonlyStateHandle<HotkeySettings>,
}

impl KeyboardInputManager {
    pub fn new(
        tx: mpsc::Sender<bool>,
        settings: ThreadedReadonlyStateHandle<HotkeySettings>,
    ) -> Self {
        KeyboardInputManager { tx, settings }
    }

    pub fn listen(mut self) {
        if let Err(error) = rdev::listen(move |e| self.callback(&e)) {
            println!("Hotkey hook error: {error:?}");
        }
    }

    fn callback(&mut self, event: &rdev::Event) {
        let Some(hotkey) = self.settings.read().key.to_rdev() else {
            return;
        };

        match event.event_type {
            rdev::EventType::KeyPress(key) if hotkey == key => {
                self.tx.send(true).unwrap();
            }
            rdev::EventType::KeyRelease(key) if hotkey == key => {
                self.tx.send(false).unwrap();
            }
            _ => {}
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SelectedButtonState {
    Selected(Button),
    #[default]
    Disabled,
    Choosing,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HotkeySettings {
    pub key: SelectableHotkey,
    pub button: SelectedButtonState,
}

pub struct HotkeyService {
    settings: ThreadedReadWriteStateHandle<HotkeySettings>,
    gamepad_rx: EventReceiver<GamepadEvent>,
    overlay_tx: mpsc::Sender<bool>,
}

impl HotkeyService {
    pub fn new(gamepad_service: &mut GamepadService, overlay_tx: &mpsc::Sender<bool>) -> Self {
        let settings = ThreadedReadWriteStateHandle::new(load_service_config(DATA_ID));

        let settings_for_kb_manager = settings.clone();
        let overlay_tx_for_kb_manager = overlay_tx.clone();
        thread::spawn(move || {
            let manager = KeyboardInputManager::new(
                overlay_tx_for_kb_manager,
                ThreadedReadonlyStateHandle::over(&settings_for_kb_manager),
            );
            manager.listen();
        });

        HotkeyService {
            settings,
            overlay_tx: overlay_tx.clone(),
            gamepad_rx: gamepad_service.subscribe(),
        }
    }

    pub fn settings_handle(&self) -> ThreadedReadWriteStateHandle<HotkeySettings> {
        ThreadedReadWriteStateHandle::clone(&self.settings)
    }
}

const DATA_ID: &str = "hotkey_settings";

impl Service for HotkeyService {
    fn update(&mut self) {
        let mut settings = self.settings.write();
        while let Some(event) = self.gamepad_rx.try_recv() {
            match event.as_ref() {
                GamepadEvent::ButtonPressed(button) => {
                    if matches!(settings.button, SelectedButtonState::Choosing) {
                        settings.button = SelectedButtonState::Selected(*button);
                        return;
                    }

                    if SelectedButtonState::Selected(*button) == settings.button {
                        self.overlay_tx.send(true).unwrap();
                    }
                }
                GamepadEvent::ButtonReleased(button) => {
                    if SelectedButtonState::Selected(*button) == settings.button {
                        self.overlay_tx.send(false).unwrap();
                    }
                }
            }
        }
    }

    fn save(&self) {
        save_service_config(DATA_ID, &*self.settings.read());
    }
}

impl ServiceWithUi for HotkeyService {
    fn panel(&self) -> impl Panel + 'static {
        HotkeySettingsWidget::new(self)
    }
}
