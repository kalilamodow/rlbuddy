use aes::Aes256;
use base64::{Engine, engine::general_purpose};
use cipher::{BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};
use serde::{Deserialize, Serialize, de::Visitor};

#[derive(Debug, Clone)]
pub struct RlAesKey(Aes256, [u8; 32]);

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

    pub fn to_base64(&self) -> String {
        general_purpose::STANDARD.encode(self.1)
    }

    pub fn from_base64(b64: &str) -> anyhow::Result<Self> {
        let aes_key: [u8; 32] = general_purpose::STANDARD
            .decode(b64)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("decoded vec is not 32 bytes"))?;
        let cipher = Aes256::new((&aes_key).into());
        Ok(Self(cipher, aes_key))
    }
}

impl Serialize for RlAesKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_base64())
    }
}

impl<'de> Deserialize<'de> for RlAesKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RlAesKeyVisitor;
        impl<'de> Visitor<'de> for RlAesKeyVisitor {
            type Value = RlAesKey;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a base64 aes key")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                RlAesKey::from_base64(v).map_err(|e| serde::de::Error::custom(e))
            }
        }

        deserializer.deserialize_str(RlAesKeyVisitor)
    }
}
