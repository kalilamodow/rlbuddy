use crate::common::eventsource::{EventReceiver, EventSource};
use crate::common::{ReadWriteStateHandle, ReadonlyStateHandle};
use crate::core::app::Service;
use gilrs::ev::state::ButtonData;
use gilrs::{Axis, Button, EventType, Gamepad, Gilrs};

pub enum GamepadEvent {
    ButtonPressed(Button),
    ButtonReleased(Button),
}

#[derive(Debug, Clone)]
pub struct GamepadState {
    pub trigger_left: bool,
    pub trigger_right: bool,
    pub bumper_left: bool,
    pub bumper_right: bool,
    pub north: bool,
    pub south: bool,
    pub west: bool,
    pub east: bool,
    pub joy_left_x: f32,
    pub joy_left_y: f32,
    pub joy_right_x: f32,
    pub joy_right_y: f32,
}

pub type GamepadStateHandle = ReadonlyStateHandle<Option<GamepadState>>;

impl GamepadState {
    fn read_gamepad(gp: &Gamepad) -> Self {
        Self {
            trigger_left: gp
                .button_data(Button::LeftTrigger2)
                .map(ButtonData::is_pressed)
                .unwrap_or_default(),
            trigger_right: gp
                .button_data(Button::RightTrigger2)
                .map(ButtonData::is_pressed)
                .unwrap_or_default(),
            bumper_left: gp
                .button_data(Button::LeftTrigger)
                .map(ButtonData::is_pressed)
                .unwrap_or_default(),
            bumper_right: gp
                .button_data(Button::RightTrigger)
                .map(ButtonData::is_pressed)
                .unwrap_or_default(),
            north: gp
                .button_data(Button::North)
                .map(ButtonData::is_pressed)
                .unwrap_or_default(),
            south: gp
                .button_data(Button::South)
                .map(ButtonData::is_pressed)
                .unwrap_or_default(),
            west: gp
                .button_data(Button::West)
                .map(ButtonData::is_pressed)
                .unwrap_or_default(),
            east: gp
                .button_data(Button::East)
                .map(ButtonData::is_pressed)
                .unwrap_or_default(),
            joy_left_x: gp.value(Axis::LeftStickX),
            joy_left_y: gp.value(Axis::LeftStickY),
            joy_right_x: gp.value(Axis::RightStickX),
            joy_right_y: gp.value(Axis::RightStickY),
        }
    }
}

pub struct GamepadService {
    publisher: EventSource<GamepadEvent>,
    gamepad_state: ReadWriteStateHandle<Option<GamepadState>>,
    gilrs: Gilrs,
}

impl GamepadService {
    pub fn new() -> Self {
        Self {
            publisher: EventSource::new(),
            gilrs: Gilrs::new().unwrap(),
            gamepad_state: ReadWriteStateHandle::default(),
        }
    }

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
                    self.publisher.publish(GamepadEvent::ButtonPressed(button))
                }
                EventType::ButtonReleased(button, _) => {
                    self.publisher.publish(GamepadEvent::ButtonReleased(button))
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
