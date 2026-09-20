use crate::common::savedata::{load_service_data, save_service_data};
use aes::Aes256;
use base64::{Engine, engine::general_purpose};
use cipher::{BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

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

    fn from_base64(b64: &str) -> Self {
        let aes_key: [u8; 32] = general_purpose::STANDARD
            .decode(b64)
            .unwrap()
            .try_into()
            .unwrap();
        let cipher = Aes256::new((&aes_key).into());
        Self(cipher)
    }
}

const URL: &str =
    "https://raw.githubusercontent.com/ShinyEmii/Toga-Files/refs/heads/master/aes.txt";
const DATA_ID: &str = "swapper-encryption";

#[derive(Debug, Serialize, Deserialize)]
struct AESKeyListCache {
    etag: String,
    b64s: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct EncryptionSavedata {
    cache: Option<AESKeyListCache>,
}

#[derive(Debug)]
pub enum KeyLoadingError {
    NoInternet,
}

pub fn load_all_keys() -> Result<Vec<RlAesKey>, KeyLoadingError> {
    let mut savedata: EncryptionSavedata = load_service_data(DATA_ID);

    let start = Instant::now();
    let mut request = ureq::get(URL)
        .config()
        .timeout_global(Some(Duration::from_secs(2)))
        .build();
    if let Some(cache) = &savedata.cache {
        request = request.header("if-none-match", &cache.etag);
    }

    // 304 means the cache matched
    if let Ok(response) = request.call()
        && response.status() != 304
    {
        println!("keys.txt loading took {:#?}", start.elapsed());
        let etag = String::from_utf8_lossy(response.headers().get("ETag").unwrap().as_bytes())
            .into_owned();
        let response_text = response.into_body().read_to_string().unwrap();
        let new_cache = AESKeyListCache {
            etag,
            b64s: response_text,
        };
        savedata.cache = Some(new_cache);
        save_service_data(DATA_ID, &savedata);
    }

    savedata
        .cache
        .map(|cache| {
            cache
                .b64s
                .lines()
                .map(|line| RlAesKey::from_base64(line))
                .collect()
        })
        .ok_or(KeyLoadingError::NoInternet)
}
