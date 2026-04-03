use base64::{Engine, engine::general_purpose};
use fw_error::{FwError, FwResult};

// 没有 / 和 =
#[inline]
pub fn encode_for_url<T: AsRef<[u8]>>(value: T) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(value)
}

#[inline]
pub fn decode_for_url<T: AsRef<[u8]>>(value: T) -> FwResult<Vec<u8>> {
    general_purpose::URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|e| FwError::CryptoError("base64 decode", e.to_string()))
}

#[inline]
pub fn encode<T: AsRef<[u8]>>(value: T) -> String {
    general_purpose::STANDARD.encode(value)
}

#[inline]
pub fn decode<T: AsRef<[u8]>>(value: T) -> FwResult<Vec<u8>> {
    general_purpose::STANDARD
        .decode(value)
        .map_err(|e| FwError::CryptoError("base64 decode", e.to_string()))
}
