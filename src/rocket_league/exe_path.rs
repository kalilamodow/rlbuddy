use crate::common::savedata::{load_service_data, save_service_data};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

const DATA_ID: &str = "exe-path";

#[derive(Debug, Default, Serialize, Deserialize)]
struct Cache {
    path: Option<PathBuf>,
}

static RL_EXE_PATH: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| {
    let current: Cache = load_service_data(DATA_ID);
    Mutex::new(current.path)
});

pub fn get_rl_exe_path() -> Option<PathBuf> {
    let guard = RL_EXE_PATH.lock().unwrap();
    guard.clone()
}

pub fn set_rl_exe_path(new: PathBuf) {
    let new_cache = Cache {
        path: Some(new.clone()),
    };
    save_service_data(DATA_ID, &new_cache);

    let mut guard = RL_EXE_PATH.lock().unwrap();
    *guard = Some(new);
}
