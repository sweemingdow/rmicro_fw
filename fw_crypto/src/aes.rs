use fw_error::{FwError, FwResult};

pub mod cbc;
pub mod ecb;
pub mod gcm;

pub enum AesBitsType {
    Bits256,
    Bits128,
}

pub fn into_plain(desc: &'static str, plains: Vec<u8>) -> FwResult<String> {
    String::from_utf8(plains)
        .map_err(|e| FwError::CryptoError(desc, format!("invalid decrypted vec data, err={}", e)))
}
