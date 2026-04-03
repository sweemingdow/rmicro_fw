use crate::b64;
use crate::hex::hex_encode;
use aes::cipher::crypto_common::rand_core::OsRng;
use aes::{
    Aes128, Aes256,
    cipher::{BlockDecryptMut, BlockEncryptMut, block_padding::Pkcs7},
};
use ecb::cipher::KeyInit;
use fw_error::{FwError, FwResult};

type Aes256EcbEnc = ecb::Encryptor<Aes256>;
type Aes256EcbDec = ecb::Decryptor<Aes256>;
type Aes128EcbEnc = ecb::Encryptor<Aes128>;
type Aes128EcbDec = ecb::Decryptor<Aes128>;

/// AES-256-ECB 加密（底层字节版本）
pub fn ecb_256_encrypt_floor(key: &[u8], plains: &[u8]) -> FwResult<Vec<u8>> {
    let cipher = Aes256EcbEnc::new(key.into());
    let mut buffer = plains.to_vec();

    let block_size = 16;
    let padding_len = block_size - (buffer.len() % block_size);
    buffer.resize(buffer.len() + padding_len, 0);

    // 加密
    cipher
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, plains.len())
        .map_err(|e| FwError::CryptoError("ecb_256_encrypt", e.to_string()))?;

    Ok(buffer)
}

pub fn ecb_256_encrypt(key: &[u8], plains: &[u8]) -> FwResult<String> {
    let ciphertext = ecb_256_encrypt_floor(key, plains)?;
    Ok(b64::encode(&ciphertext))
}

pub fn ecb_256_decrypt_floor(key: &[u8], ciphertext: &[u8]) -> FwResult<Vec<u8>> {
    let cipher = Aes256EcbDec::new(key.into());
    let mut buffer = ciphertext.to_vec();

    let decrypted = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|e| FwError::CryptoError("ecb_256_decrypt", e.to_string()))?;

    Ok(decrypted.to_vec())
}

pub fn ecb_256_decrypt(key: &[u8], ciphertext_b64: &str) -> FwResult<String> {
    let ciphertext = b64::decode(ciphertext_b64)?;
    let plaintext = ecb_256_decrypt_floor(key, &ciphertext)?;

    String::from_utf8(plaintext).map_err(|e| FwError::CryptoError("ecb_256_decrypt", e.to_string()))
}

/// AES-128-ECB 加密（底层字节版本）
pub fn ecb_128_encrypt_floor(key: &[u8], plains: &[u8]) -> FwResult<Vec<u8>> {
    let cipher = Aes128EcbEnc::new(key.into());
    let mut buffer = plains.to_vec();

    let block_size = 16;
    let padding_len = block_size - (buffer.len() % block_size);
    buffer.resize(buffer.len() + padding_len, 0);

    cipher
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, plains.len())
        .map_err(|e| FwError::CryptoError("ecb_128_encrypt", e.to_string()))?;

    Ok(buffer)
}

pub fn ecb_128_encrypt(key: &[u8], plains: &[u8]) -> FwResult<String> {
    let ciphertext = ecb_128_encrypt_floor(key, plains)?;
    Ok(b64::encode(&ciphertext))
}

pub fn ecb_128_decrypt_floor(key: &[u8], ciphertext: &[u8]) -> FwResult<Vec<u8>> {
    let cipher = Aes128EcbDec::new(key.into());
    let mut buffer = ciphertext.to_vec();

    let decrypted = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|e| FwError::CryptoError("ecb_128_decrypt", e.to_string()))?;

    Ok(decrypted.to_vec())
}

pub fn ecb_128_decrypt(key: &[u8], ciphertext_b64: &str) -> FwResult<String> {
    let ciphertext = b64::decode(ciphertext_b64)?;
    let plaintext = ecb_128_decrypt_floor(key, &ciphertext)?;

    String::from_utf8(plaintext).map_err(|e| FwError::CryptoError("ecb_128_decrypt", e.to_string()))
}

pub fn gen_ecb_256_key_with_hex() -> String {
    let key = Aes256EcbEnc::generate_key(OsRng);
    hex_encode(key)
}

pub fn gen_ecb_128_key_with_hex() -> String {
    let key = Aes128EcbEnc::generate_key(OsRng);
    hex_encode(key)
}

pub fn gen_ecb_256_key_with_b64() -> String {
    let key = Aes256EcbEnc::generate_key(OsRng);
    b64::encode(key)
}

pub fn gen_ecb_128_key_with_b64() -> String {
    let key = Aes128EcbEnc::generate_key(OsRng);
    b64::encode(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::hex_decode;

    #[test]
    fn test_ecb_256() {
        let key_hex = gen_ecb_256_key_with_hex();
        let key = hex_decode(&key_hex).unwrap();

        let plaintext = "Hello AES-256-ECB！测试数据";
        println!("Original: {}", plaintext);

        // 加密
        let cipher_b64 = ecb_256_encrypt(&key, plaintext.as_bytes()).unwrap();
        println!("Ciphertext (base64): {}...", &cipher_b64[..50]);

        // 解密
        let decrypted = ecb_256_decrypt(&key, &cipher_b64).unwrap();
        println!("Decrypted: {}", decrypted);

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_ecb_128() {
        let key_hex = gen_ecb_128_key_with_hex();
        let key = hex_decode(&key_hex).unwrap();

        let plaintext = "Hello AES-128-ECB！";

        let cipher_b64 = ecb_128_encrypt(&key, plaintext.as_bytes()).unwrap();
        let decrypted = ecb_128_decrypt(&key, &cipher_b64).unwrap();

        assert_eq!(plaintext, decrypted);
    }
}
