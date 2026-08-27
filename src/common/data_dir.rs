use std::path::PathBuf;

pub fn rlbuddy_data_dir() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .map(|roaming| PathBuf::from(roaming).join("rlbuddy/"))
        .ok()
}
