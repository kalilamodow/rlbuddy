use crate::{
    common::{
        ReadWriteStateHandle, ReadonlyStateHandle,
        channel::{Receiver, Sender},
        savedata::{load_service_data, save_service_data},
    },
    core::app::{Service, ServiceWithUi},
    rocket_league::{ItemPackageName, ItemsLoadStatus, get_items},
    swapper::{upk::Upk, widget::SwapperWidget},
};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSwap {
    pub appearance: ItemPackageName,
    pub replaced: ItemPackageName,
}

#[derive(Debug)]
pub enum SwapperCommand {
    Swap {
        replaced: ItemPackageName,
        appearance: ItemPackageName,
    },
    DeleteSwap(ItemPackageName), // replaced
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

    fn handle_command(&mut self, command: SwapperCommand) {
        match command {
            SwapperCommand::DeleteSwap(item) => {
                let mut state = self.state.write();
                let Some(exe_path) = state.exe_path.clone() else {
                    return;
                };

                if let Err(error) = fs::remove_file(item.path(&exe_path)) {
                    state.current_error =
                        Some(format!("error when removing masquerading file: {error:?}"));
                };

                if let Err(error) = fs::copy(item.backup_path(&exe_path), item.path(&exe_path)) {
                    state.current_error =
                        Some(format!("error when copying backup file: {error:?}"));
                }

                state.active_swaps.retain(|s| s.replaced != item);
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
                    return;
                };

                if !appearance.path(&exe_path).is_file() {
                    state.current_error = Some("invalid appearance item".into());
                    return;
                }
                if !replaced.path(&exe_path).is_file() {
                    state.current_error = Some("invalid replaced item".into());
                    return;
                }

                if !replaced.backup_path(&exe_path).is_file() {
                    fs::copy(replaced.path(&exe_path), replaced.backup_path(&exe_path)).unwrap();
                }

                let ItemsLoadStatus::Loaded(items) = &*get_items() else {
                    state.current_error = Some("items aren't loaded yet".into());
                    return;
                };
                let Some(appearance_item_def) = items.iter().find(|i| i.package == appearance)
                else {
                    state.current_error = Some("invalid appearance item".into());
                    return;
                };
                let Some(replaced_item_def) = items.iter().find(|i| i.package == replaced) else {
                    state.current_error = Some("invalid replaced item".into());
                    return;
                };

                let mut appearance_upk = match Upk::open(
                    appearance.path(&exe_path),
                    &appearance,
                    &appearance_item_def.key,
                ) {
                    Ok(u) => u,
                    Err(error) => {
                        state.current_error = Some(format!("Loading appearance file: {error:?}"));
                        return;
                    }
                };
                let replaced_upk =
                    match Upk::open(replaced.path(&exe_path), &replaced, &replaced_item_def.key) {
                        Ok(u) => u,
                        Err(error) => {
                            state.current_error = Some(format!("Loading replaced file: {error:?}"));
                            return;
                        }
                    };

                appearance_upk.pretend_to_be(&replaced_upk);
                let serialized = match appearance_upk.serialize() {
                    Ok(s) => s,
                    Err(e) => {
                        state.current_error = Some(format!("serializing failure: {e:?}"));
                        return;
                    }
                };
                if let Err(error) = fs::write(replaced.path(&exe_path), serialized) {
                    state.current_error =
                        Some(format!("swap failure when writing new file: {error:?}"));
                    return;
                };

                let swap = ActiveSwap {
                    appearance,
                    replaced,
                };
                state.active_swaps.push(swap);
            }
            SwapperCommand::ClearError => {
                let mut state = self.state.write();
                state.current_error = None;
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
