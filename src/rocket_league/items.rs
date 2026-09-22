use crate::{
    common::savedata::{load_service_data, save_service_data},
    rocket_league::RlAesKey,
};
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex, MutexGuard},
    thread,
    time::Duration,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemId(u16);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ItemPackageName(String);

impl ItemPackageName {
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

// there are more but this is just items
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ItemSlot {
    Antenna,
    Body,
    Boost,
    Explosion,
    PaintFinish,
    Topper,
    Trail,
    Wheel,
}

impl ItemSlot {
    fn from_slot_str(slot: &str) -> Option<Self> {
        Some(match slot {
            "Antenna" => Self::Antenna,
            "Body" => Self::Body,
            "Rocket Boost" => Self::Boost,
            "Goal Explosion" => Self::Explosion,
            "Paint Finish" => Self::PaintFinish,
            "Topper" => Self::Topper,
            "Trail" => Self::Trail,
            "Wheels" => Self::Wheel,
            _ => return None,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Antenna => "Antenna",
            Self::Body => "Body",
            Self::Boost => "Boost",
            Self::Explosion => "Goal explosion",
            Self::PaintFinish => "Paint finish",
            Self::Topper => "Topper",
            Self::Trail => "Trail",
            Self::Wheel => "Wheels",
        }
    }
}

const URL: &str =
    "https://raw.githubusercontent.com/ShinyEmii/Toga-Files/refs/heads/master/products.csv";
const DATA_ID: &str = "items";

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemsCache {
    etag: String,
    items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub slot: ItemSlot,
    pub package: ItemPackageName,
    pub key: RlAesKey,
}

pub enum ItemsLoadStatus {
    Loaded(Vec<Item>),
    Loading,
    NotLoaded,
    Error(anyhow::Error),
}

static ITEMS: LazyLock<Mutex<ItemsLoadStatus>> =
    LazyLock::new(|| Mutex::new(ItemsLoadStatus::NotLoaded));

fn trim_quotes(string: &str) -> &str {
    &string[1..string.len() - 1]
}

fn parse_csv_response(csv: String) -> Result<Vec<Item>> {
    let mut items = Vec::new();

    // skip table header
    for line in csv.lines().skip(1) {
        let mut values = line.split(',');
        let id = ItemId(values.next().context("loading next id")?.parse()?);
        values.next(); // skip name
        let name = trim_quotes(values.next().context("loading next label")?).to_owned(); // Label

        let Some(slot) =
            ItemSlot::from_slot_str(trim_quotes(values.next().context("loading next slot")?))
        else {
            continue;
        };

        values.next(); // skip quality
        values.next(); // skip unlock method
        values.next(); // skip pack
        let package = ItemPackageName(
            trim_quotes(values.next().context("loading package value")?).to_owned(),
        );

        let key = trim_quotes(values.next().context("loading key value")?);
        if key.is_empty() {
            continue;
        }
        let key = RlAesKey::from_base64(key)?;

        items.push(Item {
            id,
            name,
            slot,
            package,
            key,
        });
    }

    Ok(items)
}

fn load_items() {
    thread::spawn(|| {
        let etag = {
            let cached: Option<ItemsCache> = load_service_data(DATA_ID); // so default is None
            let mut guard = ITEMS.lock().unwrap();
            match cached {
                Some(c) => {
                    *guard = ItemsLoadStatus::Loaded(c.items);
                    Some(c.etag)
                }
                None => {
                    *guard = ItemsLoadStatus::Loading;
                    None
                }
            }
        };

        let mut request = ureq::get(URL)
            .config()
            .timeout_global(Some(Duration::from_secs(2)))
            .build();
        if let Some(etag) = etag {
            request = request.header("if-none-match", &etag);
        }

        // 304 means the cache matched
        if let Ok(response) = request.call()
            && response.status() != 304
        {
            let etag = String::from_utf8_lossy(response.headers().get("ETag").unwrap().as_bytes())
                .into_owned();
            let response_text = response.into_body().read_to_string().unwrap();
            let response = parse_csv_response(response_text);

            match response {
                Ok(items) => {
                    let mut guard = ITEMS.lock().unwrap();
                    *guard = ItemsLoadStatus::Loaded(items.clone());

                    let new_cache = Some(ItemsCache { etag, items });
                    save_service_data(DATA_ID, &new_cache);
                }
                Err(error) => {
                    let mut guard = ITEMS.lock().unwrap();
                    *guard = ItemsLoadStatus::Error(error);
                }
            }
        }
    });
}

pub fn get_items() -> MutexGuard<'static, ItemsLoadStatus> {
    let items = ITEMS.lock().unwrap();
    if matches!(&*items, ItemsLoadStatus::NotLoaded) {
        load_items();
    }

    items
}
