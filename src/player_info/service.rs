use crate::core::app::{Panel, Service, ServiceWithUi};
use crate::player_info::PlayerSearchWidget;
use crate::{
    common::channel::{Receiver, Sender},
    rocket_league::Platform,
};

pub enum PlayerInfoServiceCommand {
    Open(Platform, String),
}

pub struct PlayerInfoService {
    command_receiver: Receiver<PlayerInfoServiceCommand>,
}

impl Default for PlayerInfoService {
    fn default() -> Self {
        Self {
            command_receiver: Receiver::new(),
        }
    }
}

impl PlayerInfoService {
    pub fn update(&mut self) {
        while let Some(command) = self.command_receiver.try_recv() {
            match command {
                PlayerInfoServiceCommand::Open(platform, platform_id) => {
                    let platform_str = match platform {
                        Platform::Bot => {
                            eprintln!("tried opening bot in trn");
                            return;
                        }
                        Platform::Epic => "epic",
                        Platform::PlayStation => "psn",
                        Platform::Steam => "steam",
                        Platform::Switch => "switch",
                        Platform::Xbox => "xbl",
                    };

                    let _ = webbrowser::open(&format!(
                        "https://tracker.gg/rocket-league/profile/{platform_str}/{}/overview",
                        urlencoding::encode(&platform_id)
                    ));
                }
            }
        }
    }

    pub fn sender(&self) -> Sender<PlayerInfoServiceCommand> {
        self.command_receiver.send()
    }
}

impl Service for PlayerInfoService {
    fn update(&mut self) {
        self.update();
    }
}

impl ServiceWithUi for PlayerInfoService {
    fn panel(&self) -> impl Panel + 'static {
        PlayerSearchWidget::new(self)
    }
}
