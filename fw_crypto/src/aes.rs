use fw_error::{FwError, FwResult};
use crate::b64;
use crate::hex::hex_decode;

pub mod cbc;
pub mod ecb;
pub mod gcm;

pub enum AesBitsType {
    Bits256,
    Bits128,
}

pub enum AesKeyDisplayType {
    Hex,
    B64,
}

impl AesKeyDisplayType {
    pub fn to_bin(&self, key: &str) -> FwResult<Vec<u8>> {
        match self {
            AesKeyDisplayType::Hex => hex_decode(key),
            AesKeyDisplayType::B64 => b64::decode(key),
        }
    }
}

