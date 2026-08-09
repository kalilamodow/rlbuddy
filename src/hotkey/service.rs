use crate::common::{ThreadedReadWriteStateHandle, ThreadedReadonlyStateHandle};
use gilrs::{Button, Gilrs};
use rdev::Key;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::mpsc, thread, time};

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
    keys_pressed: HashSet<Key>,
    was_open_before: bool,
    tx: mpsc::Sender<bool>,
    settings: ThreadedReadonlyStateHandle<HotkeySettings>,
}

impl KeyboardInputManager {
    pub fn new(
        tx: mpsc::Sender<bool>,
        settings: ThreadedReadonlyStateHandle<HotkeySettings>,
    ) -> Self {
        KeyboardInputManager {
            keys_pressed: HashSet::new(),
            was_open_before: false,
            tx,
            settings,
        }
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
            rdev::EventType::KeyPress(key) => {
                self.keys_pressed.insert(key);
            }
            rdev::EventType::KeyRelease(key) => {
                self.keys_pressed.remove(&key);
            }
            _ => {}
        }

        if self.keys_pressed.contains(&hotkey) {
            if !self.was_open_before {
                self.was_open_before = true;
                self.tx.send(true).unwrap();
            }
        } else if self.was_open_before {
            self.was_open_before = false;
            self.tx.send(false).unwrap();
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectableControllerButton {
    #[default]
    Select,
    Start,
    LeftBumper,
    RightBumper,
    Disabled,
}

impl SelectableControllerButton {
    pub fn to_gilrs_button(&self) -> Option<Button> {
        Some(match self {
            Self::Disabled => return None,
            Self::Select => Button::Select,
            Self::Start => Button::Start,
            Self::LeftBumper => Button::LeftTrigger,
            Self::RightBumper => Button::RightTrigger,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Disabled => "Disabled",
            Self::Select => "Select",
            Self::Start => "Start",
            Self::LeftBumper => "Left bumper",
            Self::RightBumper => "Right bumper",
        }
    }
}

struct ControllerInputManager {
    tx: mpsc::Sender<bool>,
    settings: ThreadedReadonlyStateHandle<HotkeySettings>,
}

impl ControllerInputManager {
    pub fn new(
        tx: mpsc::Sender<bool>,
        settings: ThreadedReadonlyStateHandle<HotkeySettings>,
    ) -> Self {
        Self { tx, settings }
    }

    pub fn listen(self) {
        let mut g = Gilrs::new().unwrap();

        loop {
            while let Some(event) = g.next_event() {
                match event.event {
                    gilrs::EventType::ButtonPressed(button, _) => {
                        if Some(button) == self.settings.read().button.to_gilrs_button() {
                            self.tx.send(true).unwrap();
                        }
                    }
                    gilrs::EventType::ButtonReleased(button, _)
                        if Some(button) == self.settings.read().button.to_gilrs_button() =>
                    {
                        self.tx.send(false).unwrap();
                    }
                    _ => {}
                }
            }

            thread::sleep(time::Duration::from_millis(5));
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HotkeySettings {
    pub key: SelectableHotkey,
    pub button: SelectableControllerButton,
}

pub struct HotkeyService {
    settings: ThreadedReadWriteStateHandle<HotkeySettings>,
}

impl HotkeyService {
    pub fn new(overlay_tx: mpsc::Sender<bool>, settings: Option<HotkeySettings>) -> Self {
        let settings = ThreadedReadWriteStateHandle::new(settings.unwrap_or_default());

        let settings_for_kb_manager = settings.clone();
        let overlay_tx_for_kb_manager = overlay_tx.clone();
        thread::spawn(move || {
            let manager = KeyboardInputManager::new(
                overlay_tx_for_kb_manager,
                ThreadedReadonlyStateHandle::over(&settings_for_kb_manager),
            );
            manager.listen();
        });

        let settings_for_ctrl_manager = settings.clone();
        let overlay_tx_for_ctrl_manager = overlay_tx.clone();
        thread::spawn(move || {
            let manager = ControllerInputManager::new(
                overlay_tx_for_ctrl_manager,
                ThreadedReadonlyStateHandle::over(&settings_for_ctrl_manager),
            );
            manager.listen();
        });

        HotkeyService { settings }
    }

    pub fn settings_handle(&self) -> ThreadedReadWriteStateHandle<HotkeySettings> {
        ThreadedReadWriteStateHandle::clone(&self.settings)
    }
}
