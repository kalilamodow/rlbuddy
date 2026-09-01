use crate::common::eventsource::EventReceiver;
use crate::common::{ThreadedReadWriteStateHandle, ThreadedReadonlyStateHandle};
use crate::gamepad::{GamepadEvent, GamepadService};
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HotkeySettings {
    pub key: SelectableHotkey,
    pub button: SelectableControllerButton,
}

pub struct HotkeyService {
    settings: ThreadedReadWriteStateHandle<HotkeySettings>,
    gamepad_rx: EventReceiver<GamepadEvent>,
    overlay_tx: mpsc::Sender<bool>,
}

impl HotkeyService {
    pub fn new(
        gamepad_service: &mut GamepadService,
        overlay_tx: &mpsc::Sender<bool>,
        settings: HotkeySettings,
    ) -> Self {
        let settings = ThreadedReadWriteStateHandle::new(settings);

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

    pub fn update(&mut self) {
        let settings = self.settings.read();
        while let Some(event) = self.gamepad_rx.try_recv() {
            match event.as_ref() {
                GamepadEvent::ButtonPressed(button) => {
                    if Some(*button) == settings.button.to_gilrs_button() {
                        self.overlay_tx.send(true).unwrap();
                    }
                }
                GamepadEvent::ButtonReleased(button) => {
                    if Some(*button) == settings.button.to_gilrs_button() {
                        self.overlay_tx.send(false).unwrap();
                    }
                }
            }
        }
    }

    pub fn settings_handle(&self) -> ThreadedReadWriteStateHandle<HotkeySettings> {
        ThreadedReadWriteStateHandle::clone(&self.settings)
    }
}
