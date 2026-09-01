use crate::common::eventsource::{EventReceiver, EventSource};
use gilrs::{EventType, Gilrs};

pub enum GamepadEvent {
    ButtonPressed(gilrs::Button),
    ButtonReleased(gilrs::Button),
}

// coming soon ;)
// pub struct GamepadState {
//     pub trigger_left: f32,
//     pub trigger_right: f32,
//     pub bumper_left: bool,
//     pub bumper_right: bool,
//     pub north: bool,
//     pub south: bool,
//     pub west: bool,
//     pub east: bool,
//     pub joy_left_x: f32,
//     pub joy_left_y: f32,
//     pub joy_right_x: f32,
//     pub joy_right_y: f32,
// }

pub struct GamepadService {
    publisher: EventSource<GamepadEvent>,
    gilrs: Gilrs,
}

impl GamepadService {
    pub fn new() -> Self {
        Self {
            publisher: EventSource::new(),
            gilrs: Gilrs::new().unwrap(),
        }
    }

    pub fn subscribe(&mut self) -> EventReceiver<GamepadEvent> {
        self.publisher.subscribe()
    }

    pub fn update(&mut self) {
        self.publish_gilrs_events();
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
}
