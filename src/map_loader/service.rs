use std::path::PathBuf;

use crate::common::{
    ReadWriteStateHandle, ReadonlyStateHandle,
    channel::{Receiver, Sender},
};
use serde::{Deserialize, Serialize};

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
    pub id: CustomMapId, // folder name
    pub author: String,  // from info.json
    pub description: String, // from info.json
                         // preview path is always preview.jpg
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
}

#[derive(Debug)]
pub enum MapLoaderCommand {
    Import(String), // zip file path
    Load(CustomMapId),
    UpdateUnderpassPath(PathBuf),
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
                println!("importing {path:?}");
            }
            MapLoaderCommand::Load(id) => {
                println!("loading {id:?}");
            }
        }
    }
}
