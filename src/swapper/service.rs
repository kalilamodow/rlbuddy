use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    common::{
        ReadWriteStateHandle, ReadonlyStateHandle,
        channel::{Receiver, Sender},
        savedata::{load_service_data, save_service_data},
    },
    core::app::{Service, ServiceWithUi},
    swapper::{encryption::load_all_keys, upk::Upk, widget::SwapperWidget},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemId(String);

impl ItemId {
    pub fn new(item_id: String) -> Self {
        Self(item_id)
    }

    pub fn id(&self) -> &str {
        &self.0
    }
    pub fn sf_name(&self) -> String {
        format!("{}_SF", self.id())
    }
    pub fn filename(&self) -> String {
        format!("{}.upk", self.sf_name())
    }
    pub fn path(&self, exe_path: &Path) -> PathBuf {
        exe_path
            .parent()
            .unwrap()
            .join("../../TAGame/CookedPCConsole/")
            .join(self.filename())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveSwap {
    appearance: ItemId,
    replaced: ItemId,
}

#[derive(Debug)]
pub enum SwapperCommand {
    Swap {
        replaced: ItemId,
        appearance: ItemId,
    },
    DeleteSwap(ItemId), // replaced
    SetExecutablePath(PathBuf),
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SwapperServiceState {
    pub active_swaps: Vec<ActiveSwap>,
    pub exe_path: Option<PathBuf>,
}

pub struct SwapperService {
    state: ReadWriteStateHandle<SwapperServiceState>,
    command_receiver: Receiver<SwapperCommand>,
}

const DATA_ID: &str = "item_swapper";

impl SwapperService {
    pub fn new() -> Self {
        Self {
            state: ReadWriteStateHandle::new(load_service_data(DATA_ID)),
            command_receiver: Receiver::new(),
        }
    }

    pub fn state_handle(&self) -> ReadonlyStateHandle<SwapperServiceState> {
        ReadonlyStateHandle::over(&self.state)
    }

    pub fn sender(&self) -> Sender<SwapperCommand> {
        self.command_receiver.send()
    }

    fn handle_command(&mut self, command: SwapperCommand) {
        match command {
            SwapperCommand::DeleteSwap(id) => println!("deleting {id:?}"),
            SwapperCommand::SetExecutablePath(path) => {
                let mut state = self.state.write();
                state.exe_path = Some(path);
            }
            SwapperCommand::Swap {
                replaced,
                appearance,
            } => {
                let Some(exe_path) = &self.state.read().exe_path else {
                    return;
                };

                let Some(keys) = load_all_keys(&exe_path) else {
                    eprintln!("failed to load keys");
                    return;
                };

                let Ok(mut file) = fs::File::open(&replaced.path(&exe_path)) else {
                    eprintln!("failed to read upk file");
                    return;
                };

                let upk = match Upk::new(&mut file, &keys) {
                    Ok(u) => u,
                    Err(error) => {
                        eprintln!("{error:?}");
                        return;
                    }
                };
            }
        }
    }
}

impl Service for SwapperService {
    fn update(&mut self) {
        while let Some(cmd) = self.command_receiver.try_recv() {
            self.handle_command(cmd);
        }
    }

    fn save(&self) {
        save_service_data(DATA_ID, &*self.state_handle().read());
    }
}

impl ServiceWithUi for SwapperService {
    fn panel(&self) -> impl crate::core::app::Panel + 'static {
        SwapperWidget::new(self)
    }
}
