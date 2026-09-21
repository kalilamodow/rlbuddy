use crate::{
    common::{
        ReadWriteStateHandle, ReadonlyStateHandle,
        channel::{Receiver, Sender},
        savedata::{load_service_data, save_service_data},
    },
    core::app::{Service, ServiceWithUi},
    swapper::{
        encryption::{KeyLoadingError, RlAesKey, load_all_keys},
        upk::Upk,
        widget::SwapperWidget,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub fn backup_filename(&self) -> String {
        format!("{}.upk.bak", self.sf_name())
    }
    pub fn path(&self, exe_path: &Path) -> PathBuf {
        exe_path
            .parent()
            .unwrap()
            .join("../../TAGame/CookedPCConsole/")
            .join(self.filename())
    }
    pub fn backup_path(&self, exe_path: &Path) -> PathBuf {
        exe_path
            .parent()
            .unwrap()
            .join("../../TAGame/CookedPCConsole/")
            .join(self.backup_filename())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSwap {
    pub appearance: ItemId,
    pub replaced: ItemId,
}

#[derive(Debug)]
pub enum SwapperCommand {
    Swap {
        replaced: ItemId,
        appearance: ItemId,
    },
    DeleteSwap(ItemId), // replaced
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
    pub keys_loaded: bool,
}

type KeysArtex = Arc<Mutex<Option<Result<Vec<RlAesKey>, KeyLoadingError>>>>;

pub struct SwapperService {
    state: ReadWriteStateHandle<SwapperServiceState>,
    keys: KeysArtex,
    command_receiver: Receiver<SwapperCommand>,
}

fn fetch_aes_keys() -> KeysArtex {
    let rc = KeysArtex::default();

    let handle = Arc::clone(&rc);
    thread::spawn(move || {
        let keys = load_all_keys();
        let mut guard = handle.lock().unwrap();
        *guard = Some(keys);
    });

    rc
}

const DATA_ID: &str = "item_swapper";

impl SwapperService {
    pub fn new() -> Self {
        let savedata: SwapperServiceSavedata = load_service_data(DATA_ID);
        let keys = fetch_aes_keys();

        Self {
            state: ReadWriteStateHandle::new(SwapperServiceState {
                active_swaps: savedata.active_swaps,
                exe_path: savedata.exe_path,
                current_error: None,
                keys_loaded: false,
            }),
            keys,
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

                let keys_guard = self.keys.lock().unwrap();
                let Some(keys) = keys_guard.as_ref() else {
                    state.current_error = Some("Updating keys...".into());
                    return;
                };

                let Ok(keys) = keys else {
                    state.current_error = Some("Unable to load keys".into());
                    return;
                };

                let mut appearance_upk =
                    match Upk::open(appearance.path(&exe_path), &appearance, keys) {
                        Ok(u) => u,
                        Err(error) => {
                            state.current_error =
                                Some(format!("Loading appearance file: {error:?}"));
                            return;
                        }
                    };
                let replaced_upk = match Upk::open(replaced.path(&exe_path), &replaced, keys) {
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
        {
            let mut state = self.state.write();
            state.keys_loaded = self.keys.lock().unwrap().is_some();
        }

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
