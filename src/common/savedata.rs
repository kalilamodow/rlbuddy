use std::fs;
use std::path::PathBuf;

pub fn rlbuddy_data_dir() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .map(|roaming| PathBuf::from(roaming).join("rlbuddy/"))
        .ok()
}

pub fn feature_json_data_path(name: &str) -> Option<PathBuf> {
    rlbuddy_data_dir().map(|d| d.join(format!("{name}.json")))
}

pub fn load_service_data<T>(name: &str) -> T
where
    T: serde::de::DeserializeOwned + Default,
{
    let Some(path) = feature_json_data_path(name) else {
        return T::default();
    };

    let Ok(string) = fs::read_to_string(path) else {
        return T::default();
    };

    serde_json::from_str(&string).unwrap_or_default()
}

pub fn save_service_data<T>(name: &str, new: T)
where
    T: serde::Serialize + Default,
{
    let string = match serde_json::to_string(&new) {
        Ok(wtv) => wtv,
        Err(e) => {
            eprintln!("Failed to serialize settings for {name}: {e:?}");
            return;
        }
    };

    let Some(path) = feature_json_data_path(name) else {
        eprintln!("Failed to write settings for {name}: no data dir");
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Err(error) = fs::write(&path, string) {
        eprintln!("Failed to write settings for {name}: {error:?} ({path:?})");
    }
}
