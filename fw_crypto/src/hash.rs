use crate::hex::{hex_decode, hex_encode};
use fw_error::{FwError, FwResult};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};

#[inline]
pub fn hash_sign_floor<D: Digest>(val: &[u8]) -> Vec<u8> {
    let mut hasher = D::new();
    hasher.update(val);
    hasher.finalize().to_vec()
}

#[inline]
pub fn hash_sign<D: Digest>(val: &[u8]) -> String {
    let mut hasher = D::new();
    hasher.update(val);
    hex_encode(hasher.finalize().to_vec())
}

pub enum HmacAlgorithm {
    SHA1,
    SHA256,
    SHA512,
}

macro_rules! hmac_compute {
    ($key:expr, $msg:expr, $algo:ty) => {{
        let mut mac = Hmac::<$algo>::new_from_slice($key.as_bytes())
            .map_err(|e| FwError::CryptoError("hmac_create", e.to_string()))?;
        mac.update($msg.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }};
}

macro_rules! hmac_verify {
    ($key:expr, $msg:expr, $excepted_bytes:expr, $algo:ty) => {{
        let mut mac = Hmac::<$algo>::new_from_slice($key.as_bytes())
            .map_err(|e| FwError::CryptoError("hmac_create", e.to_string()))?;
        mac.update($msg.as_bytes());
        mac.verify_slice($excepted_bytes)
            .map_err(|e| FwError::CryptoError("hmac_verify", e.to_string()))
    }};
}

impl HmacAlgorithm {
    pub fn compute(&self, key: &str, msg: &str) -> FwResult<String> {
        let bytes = match self {
            HmacAlgorithm::SHA1 => hmac_compute!(key, msg, Sha1),
            HmacAlgorithm::SHA256 => hmac_compute!(key, msg, Sha256),
            HmacAlgorithm::SHA512 => hmac_compute!(key, msg, Sha512),
        };
        Ok(hex_encode(bytes))
    }

    pub fn verify(&self, key: &str, msg: &str, excepted: &str) -> FwResult<()> {
        let excepted_bytes = hex_decode(excepted)?;

        match self {
            HmacAlgorithm::SHA1 => hmac_verify!(key, msg, &excepted_bytes, Sha1),
            HmacAlgorithm::SHA256 => hmac_verify!(key, msg, &excepted_bytes, Sha256),
            HmacAlgorithm::SHA512 => hmac_verify!(key, msg, &excepted_bytes, Sha512),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash() {
        let v = hash_sign::<sha2::Sha256>("0".as_bytes());
        println!("{v}");

        let v = hash_sign::<sha1::Sha1>("0".as_bytes());
        println!("{v}");
    }

    #[test]
    fn test_hmac_sha256_verify() {
        let excepted = "4b4db2d8e749b1b8beff83d328665055cf0d52b35ca32703e3c05993eed4f7e4";
        crate::hash::HmacAlgorithm::SHA256
            .verify("0", "0", excepted)
            .unwrap();
    }
}
