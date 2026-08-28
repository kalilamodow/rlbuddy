use crate::{
    common::{
        ThreadedReadWriteStateHandle, ThreadedReadonlyStateHandle,
        channel::{Receiver, Sender},
        data_dir::rlbuddy_data_dir,
    },
    map_loader::service::MapLoaderCommand::ClearError,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read as _},
    path::{Path, PathBuf},
    thread,
};
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMapId(String); // folder name

impl std::ops::Deref for CustomMapId {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMapInfo {
    pub id: CustomMapId,        // name
    pub author: Option<String>, // from info.json
    pub description: Option<String>, // from info.json
                                // preview path is always preview.jpg
}

#[derive(Debug, Deserialize)]
struct CustomMapInfoJson {
    pub author: String,
    pub desc: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapLoaderServiceSavedata {
    pub maps: Vec<CustomMapInfo>,
    pub loaded_map: Option<CustomMapId>,
    pub underpass_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MapLoaderServiceState {
    pub maps: Vec<CustomMapInfo>,
    pub loaded_map: Option<CustomMapId>,
    pub underpass_path: Option<PathBuf>,
    pub current_error: Option<String>,
    pub import_progress: Option<f32>, // 0-1
}

#[derive(Debug)]
pub enum MapLoaderCommand {
    Import(PathBuf), // zip file path
    Load(CustomMapId),
    Delete(CustomMapId),
    Unload,
    UpdateUnderpassPath(PathBuf),
    ClearError,
}

pub struct MapLoaderService {
    state: ThreadedReadWriteStateHandle<MapLoaderServiceState>,
    command_receiver: Receiver<MapLoaderCommand>,
}

impl MapLoaderService {
    pub fn new(savedata: MapLoaderServiceSavedata) -> Self {
        let underpass_path = {
            savedata
                .underpass_path
                .map(PathBuf::from)
                .take_if(|p| p.exists())
        };

        Self {
            state: ThreadedReadWriteStateHandle::new(MapLoaderServiceState {
                maps: savedata.maps,
                loaded_map: savedata.loaded_map,
                underpass_path,
                current_error: None,
                import_progress: None,
            }),
            command_receiver: Receiver::new(),
        }
    }

    pub fn sender(&self) -> Sender<MapLoaderCommand> {
        self.command_receiver.send()
    }

    pub fn state_handle(&self) -> ThreadedReadonlyStateHandle<MapLoaderServiceState> {
        ThreadedReadonlyStateHandle::over(&self.state)
    }

    pub fn save(&self) -> MapLoaderServiceSavedata {
        let state = self.state.read();
        MapLoaderServiceSavedata {
            maps: state.maps.clone(),
            loaded_map: state.loaded_map.clone(),
            underpass_path: state
                .underpass_path
                .clone()
                .and_then(|p| p.to_str().map(str::to_owned)),
        }
    }

    pub fn update(&self) {
        while let Some(command) = self.command_receiver.try_recv() {
            self.handle_command(command);
        }
    }

    fn handle_command(&self, command: MapLoaderCommand) {
        match command {
            MapLoaderCommand::UpdateUnderpassPath(path) => {
                let mut state = self.state.write();
                state.underpass_path = Some(path);
            }
            MapLoaderCommand::Import(path) => {
                if !path.extension().is_some_and(|ext| ext == "zip") || !path.is_file() {
                    return;
                }

                self.import(path);
            }
            MapLoaderCommand::Delete(id) => {
                if let Err(error) = self.delete(id) {
                    let mut state = self.state.write();
                    state.current_error = Some(error.to_string());
                }
            }
            MapLoaderCommand::Load(id) => {
                if let Err(error) = self.load(id) {
                    let mut state = self.state.write();
                    state.current_error = Some(error.to_string());
                }
            }
            MapLoaderCommand::Unload => {
                if let Err(error) = self.unload() {
                    let mut state = self.state.write();
                    state.current_error = Some(error.to_string());
                }
            }
            ClearError => {
                let mut state = self.state.write();
                state.current_error = None;
            }
        }
    }

    fn import(&self, zip_path: PathBuf) {
        let state_handle = self.state.clone();
        thread::spawn(move || {
            let result = import_blocking(&zip_path, state_handle.clone());
            if let Err(error) = result {
                let mut state = state_handle.write();
                state.current_error = Some(error.to_string());
            }
        });
    }

    fn load(&self, id: CustomMapId) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = self.state.write();
        let Some(underpass_path) = &state.underpass_path else {
            return Err(string_to_error("no valid underpass path"));
        };

        back_up_old_underpass(underpass_path)?;

        let map_directory = get_custom_map_directory(&id)?;
        fs::copy(map_directory.join("map.upk"), underpass_path)?;

        state.loaded_map = Some(id.clone());
        Ok(())
    }

    fn unload(&self) -> io::Result<()> {
        let mut state = self.state.write();
        let Some(underpass_path) = &state.underpass_path else {
            return Err(io::Error::new(io::ErrorKind::Other, "no underpass path"));
        };

        fs::remove_file(underpass_path)?;
        let backup_path = underpass_path.join("..\\Labs_Underpass_P.upk.bak");
        fs::rename(backup_path, underpass_path)?;

        state.loaded_map = None;
        Ok(())
    }

    fn delete(&self, id: CustomMapId) -> Result<(), Box<dyn std::error::Error>> {
        fs::remove_dir_all(get_custom_map_directory(&id)?)?;

        let mut state = self.state.write();
        state.maps.retain(|map| map.id.as_str() != id.as_str());

        Ok(())
    }
}

fn import_blocking(
    zip_path: &Path,
    state_handle: ThreadedReadWriteStateHandle<MapLoaderServiceState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let update_progress = |progress: f32| {
        let mut state = state_handle.write();
        state.import_progress = Some(progress);
    };

    update_progress(0.0);

    let map_name = zip_path
        .file_prefix()
        .and_then(|f| f.to_str())
        .ok_or_else(|| string_to_error("could not get map name"))?
        .to_owned();

    let file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    update_progress(0.25);

    let mut info = CustomMapInfo {
        id: CustomMapId(map_name),
        description: None,
        author: None,
    };
    let mut rl_pkg_data = vec![];
    let mut preview_image_data = vec![];

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let Some(path) = file.enclosed_name() else {
            eprintln!("failed to get enclosed name of {}", file.name());
            continue;
        };
        let Some(filename) = path.file_name().and_then(|f| f.to_str()) else {
            eprintln!("failed to get filename of {}", file.name());
            continue;
        };

        if filename == "info.json" {
            let info_json: CustomMapInfoJson = serde_json::from_reader(file)?;
            info.author = Some(info_json.author);
            info.description = Some(info_json.desc);
        } else if filename.ends_with(".udk") || filename.ends_with(".upk") {
            let total_size = usize::try_from(file.size())?;
            rl_pkg_data = vec![0u8; total_size];

            let chunk_size = total_size / 100;
            for (i, chunk) in rl_pkg_data.chunks_mut(chunk_size).enumerate() {
                file.read_exact(chunk)?;
                // i as f32 / 100.0 = actual completion
                // * 0.75 to take up the remaining 75% (starts at 25%)
                update_progress((i as f32 / 100.0) * 0.75 + 0.25);
            }
        } else if filename == "preview.jpg" {
            file.read_to_end(&mut preview_image_data)?;
        }
    }

    if rl_pkg_data.is_empty() {
        return Err(string_to_error("failed to load package data"));
    }
    // its ok if theres no preview

    let custom_map_dir = get_custom_map_directory(&info.id)?;
    fs::create_dir_all(&custom_map_dir)?;

    fs::write(custom_map_dir.join("map.upk"), rl_pkg_data)?;
    if !preview_image_data.is_empty() {
        fs::write(
            custom_map_dir.join(format!("preview.jpg")),
            preview_image_data,
        )?;
    }

    {
        let mut state = state_handle.write();
        state.maps.push(info);
        state.import_progress = None;
    }

    Ok(())
}

fn get_custom_map_directory(id: &CustomMapId) -> Result<PathBuf, String> {
    let Some(data_dir) = rlbuddy_data_dir() else {
        return Err("no data directory".into());
    };

    Ok(data_dir.join("custom maps\\").join(id.as_str()))
}

fn back_up_old_underpass(underpass_path: &Path) -> io::Result<u64> {
    let backup_path = underpass_path.join("..\\Labs_Underpass_P.upk.bak");
    if backup_path.is_file() {
        return Ok(0);
    }

    fs::copy(underpass_path, backup_path)
}

fn string_to_error(s: &str) -> Box<dyn std::error::Error> {
    Box::<dyn std::error::Error>::from(s)
}
