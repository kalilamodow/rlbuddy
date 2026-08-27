use crate::{
    common::{
        ReadWriteStateHandle, ReadonlyStateHandle,
        channel::{Receiver, Sender},
        data_dir::rlbuddy_data_dir,
    },
    map_loader::service::MapLoaderCommand::ClearError,
};
use serde::{Deserialize, Serialize};
use std::{fs, io::Read as _, path::PathBuf};
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
    pub id: CustomMapId, // name
    pub author: String,  // from info.json
    pub description: String, // from info.json
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
}

#[derive(Debug)]
pub enum MapLoaderCommand {
    Import(PathBuf), // zip file path
    Load(CustomMapId),
    UpdateUnderpassPath(PathBuf),
    ClearError,
}

pub struct MapLoaderService {
    state: ReadWriteStateHandle<MapLoaderServiceState>,
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
            state: ReadWriteStateHandle::new(MapLoaderServiceState {
                maps: savedata.maps,
                loaded_map: savedata.loaded_map,
                underpass_path,
                current_error: None,
            }),
            command_receiver: Receiver::new(),
        }
    }

    pub fn sender(&self) -> Sender<MapLoaderCommand> {
        self.command_receiver.send()
    }

    pub fn state_handle(&self) -> ReadonlyStateHandle<MapLoaderServiceState> {
        ReadonlyStateHandle::over(&self.state)
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

                if let Err(error) = self.import(path) {
                    let mut state = self.state.write();
                    state.current_error = Some(error.to_string());
                }
            }
            MapLoaderCommand::Load(id) => {
                println!("loading {id:?}");
            }
            ClearError => {
                let mut state = self.state.write();
                state.current_error = None;
            }
        }
    }

    fn import(&self, zip_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let map_name = zip_path
            .file_prefix()
            .and_then(|f| f.to_str())
            .ok_or_else(|| string_to_error("could not get map name"))?
            .to_owned();

        let file = fs::File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        let mut info: Option<CustomMapInfo> = None;
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
                if info.is_some() {
                    return Err(string_to_error("multiple custom map manifests found"));
                }

                let info_json: CustomMapInfoJson = serde_json::from_reader(file)?;
                info = Some(CustomMapInfo {
                    id: CustomMapId(map_name.clone()),
                    author: info_json.author,
                    description: info_json.desc,
                });
            } else if filename.ends_with(".udk") || filename.ends_with(".upk") {
                file.read_to_end(&mut rl_pkg_data)?;
            } else if filename == "preview.jpg" {
                file.read_to_end(&mut preview_image_data)?;
            }
        }

        let Some(info) = info else {
            return Err(string_to_error("failed to load info.json"));
        };
        if rl_pkg_data.is_empty() {
            return Err(string_to_error("failed to load package data"));
        }
        // its ok if theres no preview

        let Some(data_dir) = rlbuddy_data_dir() else {
            return Err(string_to_error("no data directory"));
        };
        let custom_map_dir = data_dir.join("custom maps\\").join(map_name);
        fs::create_dir_all(&custom_map_dir)?;

        fs::write(custom_map_dir.join("map.upk"), rl_pkg_data)?;
        if !preview_image_data.is_empty() {
            fs::write(
                custom_map_dir.join(format!("preview.jpg")),
                preview_image_data,
            )?;
        }

        {
            let mut state = self.state.write();
            state.maps.push(info);
        }

        Ok(())
    }
}

fn string_to_error(s: &str) -> Box<dyn std::error::Error> {
    Box::<dyn std::error::Error>::from(s)
}
