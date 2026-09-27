use crate::common::savedata::{load_service_config, save_service_config};
use crate::core::app::{Panel, Service, ServiceWithUi};
use crate::map_loader::widget::MapLoaderWidget;
use crate::rocket_league::get_rl_exe_path;
use crate::{
    common::{
        ThreadedReadWriteStateHandle, ThreadedReadonlyStateHandle, savedata::rlbuddy_data_dir,
    },
    map_loader::service::MapLoaderCommand::ClearError,
};
use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read as _},
    path::{Path, PathBuf},
    sync::mpsc,
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

impl CustomMapInfo {
    pub fn new(name: String, author: Option<String>, description: Option<String>) -> Self {
        Self {
            id: CustomMapId(name),
            author,
            description,
        }
    }
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
}

#[derive(Debug, Clone)]
pub struct MapLoaderServiceState {
    pub maps: Vec<CustomMapInfo>,
    pub loaded_map: Option<CustomMapId>,
    pub current_error: Option<String>,
    pub import_progress: Option<f32>, // 0-1
}

#[derive(Debug)]
pub enum MapLoaderCommand {
    // zip file path
    Import(PathBuf),
    ImportBytes {
        info: CustomMapInfo,
        zip_archive_bytes: Vec<u8>,
        image_jpeg_bytes: Vec<u8>,
    },
    Load(CustomMapId),
    Delete(CustomMapId),
    Unload,
    ClearError,
}

pub struct MapLoaderService {
    state: ThreadedReadWriteStateHandle<MapLoaderServiceState>,
    command_receiver: mpsc::Receiver<MapLoaderCommand>,
    command_sender: mpsc::Sender<MapLoaderCommand>,
}

const DATA_ID: &str = "map_loader_savedata";

impl MapLoaderService {
    pub fn new() -> Self {
        let savedata: MapLoaderServiceSavedata = load_service_config(DATA_ID);
        let (command_sender, command_receiver) = mpsc::channel();

        Self {
            state: ThreadedReadWriteStateHandle::new(MapLoaderServiceState {
                maps: savedata.maps,
                loaded_map: savedata.loaded_map,
                current_error: None,
                import_progress: None,
            }),
            command_sender,
            command_receiver,
        }
    }

    pub fn sender(&self) -> mpsc::Sender<MapLoaderCommand> {
        self.command_sender.clone()
    }

    pub fn state_handle(&self) -> ThreadedReadonlyStateHandle<MapLoaderServiceState> {
        ThreadedReadonlyStateHandle::over(&self.state)
    }

    pub fn update(&mut self) {
        for command in self.command_receiver.try_iter() {
            self.handle_command(command);
        }
    }

    fn handle_command(&self, command: MapLoaderCommand) {
        match command {
            MapLoaderCommand::Import(path) => {
                if path.extension().is_none_or(|ext| ext != "zip") || !path.is_file() {
                    return;
                }

                let state_handle = self.state.clone();
                thread::spawn(move || {
                    let result = import_archive_from_file(&path, &state_handle);

                    if let Err(error) = result {
                        let mut state = state_handle.write();
                        state.current_error = Some(error.to_string());
                    }
                });
            }
            MapLoaderCommand::ImportBytes {
                info,
                zip_archive_bytes,
                image_jpeg_bytes,
            } => {
                let state_handle = self.state.clone();
                thread::spawn(move || {
                    let result = import_archive_from_bytes(
                        info,
                        zip_archive_bytes,
                        &state_handle,
                        image_jpeg_bytes,
                    );

                    if let Err(error) = result {
                        let mut state = state_handle.write();
                        state.current_error = Some(error.to_string());
                    }
                });
            }
            MapLoaderCommand::Delete(id) => {
                self.maybe_set_error(self.delete(&id));
            }
            MapLoaderCommand::Load(id) => {
                self.maybe_set_error(self.load(&id));
            }
            MapLoaderCommand::Unload => {
                self.maybe_set_error(self.unload());
            }
            ClearError => {
                let mut state = self.state.write();
                state.current_error = None;
            }
        }
    }

    fn maybe_set_error<R>(&self, result: Result<R>) {
        if let Err(error) = result {
            let mut state = self.state.write();
            state.current_error = Some(format!("{:?}", error));
        }
    }

    fn load(&self, id: &CustomMapId) -> Result<()> {
        let mut state = self.state.write();
        let Some(underpass_path) = underpass_path() else {
            bail!("no valid underpass path");
        };

        back_up_old_underpass(&underpass_path).context("loading new custom map")?;

        let map_directory = get_custom_map_directory(id).context("loading new custom map")?;
        fs::copy(map_directory.join("map.upk"), underpass_path)
            .context("copying over new custom map")?;

        state.loaded_map = Some(id.clone());
        Ok(())
    }

    fn unload(&self) -> Result<()> {
        let mut state = self.state.write();
        let Some(underpass_path) = underpass_path() else {
            bail!("no underpass path");
        };

        let _ = fs::remove_file(&underpass_path); // ok to fail, probs doesnt exist
        let backup_path = underpass_path.join("..\\Labs_Underpass_P.upk.bak");
        fs::rename(backup_path, underpass_path)
            .context("moving backup underpass map back to normal")?;

        state.loaded_map = None;
        Ok(())
    }

    fn delete(&self, id: &CustomMapId) -> Result<()> {
        fs::remove_dir_all(get_custom_map_directory(id)?)?;

        let mut state = self.state.write();
        state.maps.retain(|map| map.id.as_str() != id.as_str());

        Ok(())
    }
}

impl Service for MapLoaderService {
    fn update(&mut self) {
        self.update();
    }

    fn save(&self) {
        let state = self.state.read();
        save_service_config(
            DATA_ID,
            &MapLoaderServiceSavedata {
                maps: state.maps.clone(),
                loaded_map: state.loaded_map.clone(),
            },
        );
    }
}

impl ServiceWithUi for MapLoaderService {
    fn panel(&self) -> impl Panel + 'static {
        MapLoaderWidget::new(self)
    }
}

fn import_archive_from_file(
    zip_path: &Path,
    state_handle: &ThreadedReadWriteStateHandle<MapLoaderServiceState>,
) -> Result<()> {
    let map_name = zip_path
        .file_prefix()
        .and_then(|f| f.to_str())
        .context("getting map name from zip path")?
        .to_owned();

    let file = fs::File::open(zip_path).context("reading archive from file")?;
    let archive = ZipArchive::new(file).context("parsing archive from file")?;

    import_archive(
        CustomMapInfo {
            id: CustomMapId(map_name),
            author: None,
            description: None,
        },
        archive,
        state_handle,
        None,
    )
    .context("importing archive from file")
}

fn import_archive_from_bytes(
    info: CustomMapInfo,
    archive_bytes: Vec<u8>,
    state_handle: &ThreadedReadWriteStateHandle<MapLoaderServiceState>,
    image_jpeg_bytes: Vec<u8>,
) -> Result<()> {
    let archive =
        ZipArchive::new(io::Cursor::new(archive_bytes)).context("parsing archive from bytes")?;
    import_archive(info, archive, state_handle, Some(image_jpeg_bytes))
        .context("importing archive from bytes")
}

fn import_archive<R>(
    mut info: CustomMapInfo,
    mut archive: ZipArchive<R>,
    state_handle: &ThreadedReadWriteStateHandle<MapLoaderServiceState>,
    default_preview_image_jpeg_data: Option<Vec<u8>>,
) -> Result<()>
where
    R: io::Read + io::Seek, // required for ZipArchive
{
    let update_progress = |progress: f32| {
        let mut state = state_handle.write();
        state.import_progress = Some(progress);
    };

    update_progress(0.05);

    let mut rl_pkg_data = vec![];
    let mut preview_image_data = default_preview_image_jpeg_data.unwrap_or_default();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
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
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("udk"))
            || path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("upk"))
        {
            let total_size = file.size() as usize;
            rl_pkg_data = vec![0u8; total_size];

            let chunk_size = (total_size / 100).max(1);
            for (i, chunk) in rl_pkg_data.chunks_mut(chunk_size).enumerate() {
                file.read_exact(chunk).unwrap();
                // i as f32 / 100.0 = actual completion
                // * 0.9 to only fill up to 90% + 0.05 because it starts at 5%
                update_progress((i as f32 / 100.0) * 0.9 + 0.05);
            }
        } else if filename == "preview.jpg" {
            file.read_to_end(&mut preview_image_data)?;
        }
    }

    if rl_pkg_data.is_empty() {
        bail!("no upk in archive");
    }
    // its ok if theres no preview

    let custom_map_dir =
        get_custom_map_directory(&info.id).context("getting directory to put imported map in")?;
    fs::create_dir_all(&custom_map_dir)
        .context("ensuring directory to put imported map in exists")?;

    fs::write(custom_map_dir.join("map.upk"), rl_pkg_data).context("writing imported map upk")?;
    if !preview_image_data.is_empty() {
        fs::write(custom_map_dir.join("preview.jpg"), preview_image_data)
            .context("writing imported map preview image")?;
    }

    {
        let mut state = state_handle.write();
        state.maps.push(info);
        state.import_progress = None;
    }

    Ok(())
}

fn get_custom_map_directory(id: &CustomMapId) -> Result<PathBuf> {
    let Some(data_dir) = rlbuddy_data_dir() else {
        bail!("no data directory");
    };

    Ok(data_dir.join("custom maps\\").join(id.as_str()))
}

fn underpass_path() -> Option<PathBuf> {
    let exe_path = get_rl_exe_path()?;
    Some(exe_path.join("../../../TAGame/CookedPCConsole/Labs_Underpass_P.upk"))
}

fn back_up_old_underpass(underpass_path: &Path) -> io::Result<u64> {
    let backup_path = underpass_path.join("..\\Labs_Underpass_P.upk.bak");
    if backup_path.is_file() {
        return Ok(0);
    }

    fs::copy(underpass_path, backup_path)
}
