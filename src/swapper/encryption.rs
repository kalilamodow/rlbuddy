use aes::Aes256;
use base64::{Engine, engine::general_purpose};
use cipher::{BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};
use serde::Deserialize;
use serde_json::Deserializer;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    sync::LazyLock,
};

const NORMAL_POLY: u32 = 0x04C11DB7;
const CRC32_INIT: u32 = !0x87636B;

static CRC_TABLE: LazyLock<[u32; 256]> = LazyLock::new(|| {
    std::array::from_fn(|i| {
        (0..8).fold((i as u32) << 24, |crc, _| {
            if crc & 0x80000000 != 0 {
                (crc << 1) ^ NORMAL_POLY
            } else {
                crc << 1
            }
        })
    })
});

fn crc32_chunk(chunk: &[u8]) -> u32 {
    let crc32 = chunk.iter().fold(CRC32_INIT, |crc32, &byte| {
        let index = byte ^ ((crc32 >> 24) as u8 & 0xFF);
        (crc32 << 8) ^ CRC_TABLE[index as usize]
    });
    !crc32
}

#[derive(Debug, Clone)]
pub struct RlAesKey(Aes256);

impl RlAesKey {
    pub fn encrypt(&self, buffer: &mut [u8]) {
        if buffer.len() % 16 != 0 {
            panic!("encryption: buffer size isnt divisible by 16");
        }

        for chunk in buffer.chunks_exact_mut(16) {
            self.0.encrypt_block(chunk.try_into().unwrap());
        }
    }

    pub fn decrypt(&self, buffer: &mut [u8]) {
        if buffer.len() % 16 != 0 {
            panic!("decryption: buffer size isnt divisible by 16");
        }

        for chunk in buffer.chunks_exact_mut(16) {
            self.0.decrypt_block(chunk.try_into().unwrap());
        }
    }

    fn from_content_key(content_key: &str) -> Self {
        let content_key_bytes: [u8; 32] = general_purpose::STANDARD
            .decode(content_key)
            .unwrap()
            .try_into()
            .unwrap();

        let aes_key: Vec<u8> = content_key_bytes
            .chunks_exact(4)
            .map(crc32_chunk)
            .flat_map(|c| c.to_le_bytes())
            .collect();
        let aes_key: [u8; 32] = aes_key.try_into().unwrap();
        let cipher = Aes256::new((&aes_key).into());

        Self(cipher)
    }

    fn from_array(array: &[u8; 32]) -> Self {
        Self(Aes256::new(array.into()))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PsynetContentMapEntry {
    content: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PsynetContentConfig {
    content_map: Vec<PsynetContentMapEntry>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PsynetResponse {
    content_config: PsynetContentConfig,
}

static DEFAULT_AES_KEY: LazyLock<RlAesKey> = LazyLock::new(|| {
    RlAesKey::from_array(&[
        0xC7, 0xDF, 0x6B, 0x13, 0x25, 0x2A, 0xCC, 0x71, 0x47, 0xBB, 0x51, 0xC9, 0x8A, 0xD7, 0xE3,
        0x4B, 0x7F, 0xE5, 0x00, 0xB7, 0x7F, 0xA5, 0xFA, 0xB2, 0x93, 0xE2, 0xF2, 0x4E, 0x6B, 0x17,
        0xE7, 0x79,
    ])
});

pub fn load_all_keys(rocket_league_exe_path: &Path) -> Option<Vec<RlAesKey>> {
    let cache_file = rocket_league_exe_path
        .parent()?
        .join("..")
        .join("..")
        .join("TAGame")
        .join("Cache")
        .join("WebCache")
        // cached psynet config
        .join("L3YyL0NvbmZpZy9CYXR0bGVDYXJzLy0xODg3Njk0MDgzL1Byb2QvRXBpYy9JTlQv");
    let file = File::open(cache_file).ok()?;
    let mut reader = BufReader::new(file);
    reader.skip_until('{' as u8).unwrap();
    reader.seek_relative(-1).unwrap();

    let mut de = Deserializer::from_reader(reader);
    let content_keys: Vec<String> = PsynetResponse::deserialize(&mut de)
        .unwrap()
        .content_config
        .content_map
        .into_iter()
        .map(|e| e.content)
        .collect();

    Some(
        content_keys
            .iter()
            .map(|k| RlAesKey::from_content_key(&k))
            .chain(std::iter::once(DEFAULT_AES_KEY.clone()))
            .collect(),
    )
}
