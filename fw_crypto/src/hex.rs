use fw_error::{FwError, FwResult};

#[inline]
pub fn hex_encode<T: AsRef<[u8]>>(value: T) -> String {
    hex::encode(value)
}

#[inline]
pub fn hex_decode<T: AsRef<[u8]>>(value: T) -> FwResult<Vec<u8>> {
    hex::decode(value).map_err(|e| FwError::CryptoError("hex decode", e.to_string()))
}
