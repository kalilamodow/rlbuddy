use std::sync::LazyLock;

use crate::common::eventsource::{EventReceiver, EventSource};
use crate::common::{ReadWriteStateHandle, ReadonlyStateHandle};
use crate::core::app::Service;
use gilrs::ev::state::ButtonData;
use gilrs::{Axis, Button, EventType, Gamepad, Gilrs};

pub enum GamepadEvent {
    ButtonPressed(Button),
    ButtonReleased(Button),
}

#[derive(Debug, Default, Clone)]
pub struct GamepadState {
    pub trigger_left: bool,
    pub trigger_right: bool,
    pub bumper_left: bool,
    pub bumper_right: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub north: bool,
    pub south: bool,
    pub west: bool,
    pub east: bool,
    pub joy_left_x: f32,
    pub joy_left_y: f32,
    pub joy_right_x: f32,
    pub joy_right_y: f32,
    pub select: bool,
    pub start: bool,
}

// use this instead of unwrap_or_default so it doesnt make a new object
pub static NO_GAMEPAD_STATE: LazyLock<GamepadState> = LazyLock::new(GamepadState::default);

pub type GamepadStateHandle = ReadonlyStateHandle<Option<GamepadState>>;

impl GamepadState {
    fn read_gamepad(gp: &Gamepad) -> Self {
        let button_pressed = |b| gp.button_data(b).is_some_and(ButtonData::is_pressed);

        Self {
            trigger_left: button_pressed(Button::LeftTrigger2),
            trigger_right: button_pressed(Button::RightTrigger2),
            bumper_left: button_pressed(Button::LeftTrigger),
            bumper_right: button_pressed(Button::RightTrigger),
            up: button_pressed(Button::DPadUp),
            down: button_pressed(Button::DPadDown),
            left: button_pressed(Button::DPadLeft),
            right: button_pressed(Button::DPadRight),
            north: button_pressed(Button::North),
            south: button_pressed(Button::South),
            west: button_pressed(Button::West),
            east: button_pressed(Button::East),
            joy_left_x: gp.value(Axis::LeftStickX),
            joy_left_y: gp.value(Axis::LeftStickY),
            joy_right_x: gp.value(Axis::RightStickX),
            joy_right_y: gp.value(Axis::RightStickY),
            select: button_pressed(Button::Select),
            start: button_pressed(Button::Start),
        }
    }
}

pub struct GamepadService {
    publisher: EventSource<GamepadEvent>,
    gamepad_state: ReadWriteStateHandle<Option<GamepadState>>,
    gilrs: Gilrs,
}

impl Default for GamepadService {
    fn default() -> Self {
        Self {
            publisher: EventSource::new(),
            gilrs: Gilrs::new().unwrap(),
            gamepad_state: ReadWriteStateHandle::default(),
        }
    }
}

impl GamepadService {
    pub fn subscribe(&mut self) -> EventReceiver<GamepadEvent> {
        self.publisher.subscribe()
    }

    pub fn update(&mut self) {
        self.publish_gilrs_events();
        self.update_state();
    }

    fn update_state(&mut self) {
        let mut state = self.gamepad_state.write();
        let Some((_, gamepad)) = self.gilrs.gamepads().next() else {
            *state = None;
            return;
        };

        *state = Some(GamepadState::read_gamepad(&gamepad));
    }

    fn publish_gilrs_events(&mut self) {
        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                EventType::ButtonPressed(button, _) => {
                    self.publisher.publish(GamepadEvent::ButtonPressed(button));
                }
                EventType::ButtonReleased(button, _) => {
                    self.publisher.publish(GamepadEvent::ButtonReleased(button));
                }
                _ => {}
            }
        }
    }

    pub(crate) fn gamepad_state_handle(&self) -> GamepadStateHandle {
        ReadonlyStateHandle::over(&self.gamepad_state)
    }
}

impl Service for GamepadService {
    fn update(&mut self) {
        self.update();
    }
}
