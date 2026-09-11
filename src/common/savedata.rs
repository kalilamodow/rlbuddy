use std::fs;
use std::path::PathBuf;

#[cfg(windows)]
pub fn rlbuddy_config_dir() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .map(|roaming| PathBuf::from(roaming).join("rlbuddy/"))
        .ok()
}

#[cfg(windows)]
pub fn rlbuddy_data_dir() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA")
        .map(|local| PathBuf::from(local).join("rlbuddy/"))
        .ok()
}

#[cfg(not(windows))]
pub fn rlbuddy_config_dir() -> Option<PathBuf> {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| {
            std::env::var("HOME")
                .map(PathBuf::from)
                .map(|h| h.join(".config"))
        })
        .map(|p| p.join("rlbuddy/"))
        .ok()
}

#[cfg(not(windows))]
pub fn rlbuddy_data_dir() -> Option<PathBuf> {
    std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|_| {
            std::env::var("HOME")
                .map(PathBuf::from)
                .map(|h| h.join(".local/share"))
        })
        .map(|p| p.join("rlbuddy/"))
        .ok()
}

pub fn feature_json_config_path(name: &str) -> Option<PathBuf> {
    rlbuddy_config_dir().map(|d| d.join(format!("{name}.json")))
}

pub fn feature_json_data_path(name: &str) -> Option<PathBuf> {
    rlbuddy_data_dir().map(|d| d.join(format!("{name}.json")))
}

fn load_service_stuff<T>(json_path_maybe: Option<PathBuf>) -> T
where
    T: serde::de::DeserializeOwned + Default,
{
    let Some(path) = json_path_maybe else {
        return T::default();
    };

    let Ok(string) = fs::read_to_string(path) else {
        return T::default();
    };

    serde_json::from_str(&string).unwrap_or_default()
}

fn save_service_stuff<T>(name: &str, path: Option<PathBuf>, new: &T)
where
    T: serde::Serialize + Default,
{
    let Some(path) = path else {
        eprintln!("Failed to write settings for {name}: no data dir");
        return;
    };

    let string = match serde_json::to_string(&new) {
        Ok(wtv) => wtv,
        Err(e) => {
            eprintln!("Failed to serialize settings for {name}: {e:?}");
            return;
        }
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Err(error) = fs::write(&path, string) {
        eprintln!(
            "Failed to write settings for {name}: {error:?} ({})",
            path.display()
        );
    }
}

pub fn load_service_config<T>(name: &str) -> T
where
    T: serde::de::DeserializeOwned + Default,
{
    load_service_stuff(feature_json_config_path(name))
}

pub fn load_service_data<T>(name: &str) -> T
where
    T: serde::de::DeserializeOwned + Default,
{
    load_service_stuff(feature_json_data_path(name))
}

pub fn save_service_config<T>(name: &str, new: &T)
where
    T: serde::Serialize + Default,
{
    save_service_stuff(name, feature_json_config_path(name), new);
}

pub fn save_service_data<T>(name: &str, new: &T)
where
    T: serde::Serialize + Default,
{
    save_service_stuff(name, feature_json_data_path(name), new);
}
