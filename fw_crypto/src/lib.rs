use fw_error::FwResult;
use crate::hex::hex_decode;

pub mod b64;
pub mod aes;
pub mod rsa;
pub mod hash;
pub mod hex;

pub enum KeyDisplayType {
    Hex,
    B64,
}

impl KeyDisplayType {
    pub fn to_bin(&self, key: &str) -> FwResult<Vec<u8>> {
        match self {
            KeyDisplayType::Hex => hex_decode(key),
            KeyDisplayType::B64 => b64::decode(key),
        }
    }
}
