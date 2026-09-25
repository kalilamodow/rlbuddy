use aes::{
    Aes256,
    cipher::{
        BlockCipherDecrypt as _, BlockCipherEncrypt as _, KeyInit as _, KeyIvInit as _,
        StreamCipher,
    },
};
use base64::{Engine, engine::general_purpose};
use serde::{Deserialize, Serialize, de::Visitor};

#[derive(Debug, Clone)]
pub struct RlAesKey([u8; 32]);

impl RlAesKey {
    pub fn encrypt(&self, buffer: &mut [u8]) {
        if buffer.len() % 16 != 0 {
            panic!("encryption: buffer size isnt divisible by 16");
        }

        let cipher = Aes256::new(&self.0.into());
        for chunk in buffer.chunks_exact_mut(16) {
            cipher.encrypt_block(chunk.try_into().unwrap());
        }
    }

    pub fn decrypt(&self, buffer: &mut [u8]) {
        if buffer.len() % 16 != 0 {
            panic!("decryption: buffer size isnt divisible by 16");
        }

        let cipher = Aes256::new(&self.0.into());
        for chunk in buffer.chunks_exact_mut(16) {
            cipher.decrypt_block(chunk.try_into().unwrap());
        }
    }

    pub fn ctr(&self, buffer: &mut [u8], nonce: &[u8; 12]) {
        let mut iv = [0u8; 16];
        iv[..12].copy_from_slice(nonce);

        let mut cipher = ctr::Ctr32BE::<Aes256>::new(&self.0.into(), &iv.into());
        cipher.apply_keystream(buffer);
    }

    pub fn to_base64(&self) -> String {
        general_purpose::STANDARD.encode(self.0)
    }

    pub fn from_base64(b64: &str) -> anyhow::Result<Self> {
        let aes_key: [u8; 32] = general_purpose::STANDARD
            .decode(b64)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("decoded vec is not 32 bytes"))?;
        Ok(Self(aes_key))
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
