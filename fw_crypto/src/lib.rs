use fw_error::{FwError, FwResult};
use crate::hex::hex_decode;

pub mod b64;
pub mod aes;
pub mod rsa;
pub mod hash;
pub mod hex;


pub fn into_plain(desc: &'static str, plains: Vec<u8>) -> FwResult<String> {
    String::from_utf8(plains)
        .map_err(|e| FwError::CryptoError(desc, format!("invalid decrypted vec data, err={}", e)))
}
