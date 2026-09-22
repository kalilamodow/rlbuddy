use crate::{
    common::{
        ReadWriteStateHandle, ReadonlyStateHandle,
        channel::{Receiver, Sender},
        savedata::{load_service_data, save_service_data},
    },
    core::app::{Service, ServiceWithUi},
    rocket_league::{Item, ItemId},
    swapper::{upk::Upk, widget::SwapperWidget},
};
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSwap {
    pub appearance: ItemId,
    pub replaced: ItemId,
}

#[derive(Debug)]
pub enum SwapperCommand {
    Swap { replaced: Item, appearance: Item },
    DeleteSwap(Item), // replaced
    SetExecutablePath(PathBuf),
    ClearError,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SwapperServiceSavedata {
    pub active_swaps: Vec<ActiveSwap>,
    pub exe_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SwapperServiceState {
    pub active_swaps: Vec<ActiveSwap>,
    pub exe_path: Option<PathBuf>,
    pub current_error: Option<String>,
}

pub struct SwapperService {
    state: ReadWriteStateHandle<SwapperServiceState>,
    command_receiver: Receiver<SwapperCommand>,
}

const DATA_ID: &str = "item_swapper";

impl SwapperService {
    pub fn new() -> Self {
        let savedata: SwapperServiceSavedata = load_service_data(DATA_ID);

        Self {
            state: ReadWriteStateHandle::new(SwapperServiceState {
                active_swaps: savedata.active_swaps,
                exe_path: savedata.exe_path,
                current_error: None,
            }),
            command_receiver: Receiver::new(),
        }
    }

    pub fn state_handle(&self) -> ReadonlyStateHandle<SwapperServiceState> {
        ReadonlyStateHandle::over(&self.state)
    }

    pub fn sender(&self) -> Sender<SwapperCommand> {
        self.command_receiver.send()
    }

    fn handle_command(&mut self, command: SwapperCommand) -> anyhow::Result<()> {
        match command {
            SwapperCommand::DeleteSwap(item) => {
                let mut state = self.state.write();
                let Some(exe_path) = state.exe_path.clone() else {
                    bail!("no exe path");
                };

                fs::remove_file(item.package.path(&exe_path))
                    .context("removing masquerading file")?;
                fs::copy(
                    item.package.backup_path(&exe_path),
                    item.package.path(&exe_path),
                )
                .context("restoring backup")?;

                state.active_swaps.retain(|s| s.replaced != item.id);
            }
            SwapperCommand::SetExecutablePath(path) => {
                let mut state = self.state.write();
                state.exe_path = Some(path);
            }
            SwapperCommand::Swap {
                replaced,
                appearance,
            } => {
                let mut state = self.state.write();
                let Some(exe_path) = state.exe_path.clone() else {
                    bail!("no exe path");
                };

                if !appearance.package.path(&exe_path).is_file() {
                    bail!("invalid appearance item");
                }
                if !replaced.package.path(&exe_path).is_file() {
                    bail!("invalid replaced item");
                }

                if !replaced.package.backup_path(&exe_path).is_file() {
                    fs::copy(
                        replaced.package.path(&exe_path),
                        replaced.package.backup_path(&exe_path),
                    )
                    .context("backing up file")?;
                }

                let mut appearance_upk = match Upk::open(
                    appearance.package.path(&exe_path),
                    &appearance.package,
                    &appearance.key,
                ) {
                    Ok(u) => u,
                    Err(error) => {
                        bail!("Loading appearance file: {error:?}");
                    }
                };
                let replaced_upk = match Upk::open(
                    replaced.package.path(&exe_path),
                    &replaced.package,
                    &replaced.key,
                ) {
                    Ok(u) => u,
                    Err(error) => {
                        bail!("Loading replaced file: {error:?}");
                    }
                };

                appearance_upk.pretend_to_be(&replaced_upk);
                let serialized = match appearance_upk.serialize() {
                    Ok(s) => s,
                    Err(e) => {
                        bail!("serializing failure: {e:?}");
                    }
                };
                if let Err(error) = fs::write(replaced.package.path(&exe_path), serialized) {
                    bail!("swap failure when writing new file: {error:?}");
                };

                let swap = ActiveSwap {
                    appearance: appearance.id,
                    replaced: replaced.id,
                };
                state.active_swaps.push(swap);
            }
            SwapperCommand::ClearError => {
                let mut state = self.state.write();
                state.current_error = None;
            }
        }

        Ok(())
    }
}

impl Service for SwapperService {
    fn update(&mut self) {
        while let Some(cmd) = self.command_receiver.try_recv() {
            if let Err(error) = self.handle_command(cmd) {
                let mut state = self.state.write();
                state.current_error = Some(error.to_string());
            }
        }
    }

    fn save(&self) {
        let state = self.state.read().clone();
        save_service_data(
            DATA_ID,
            &SwapperServiceSavedata {
                active_swaps: state.active_swaps,
                exe_path: state.exe_path,
            },
        );
    }
}

impl ServiceWithUi for SwapperService {
    fn panel(&self) -> impl crate::core::app::Panel + 'static {
        SwapperWidget::new(self)
    }
}
