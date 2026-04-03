use crate::aes::{ AesBitsType, AesKeyDisplayType};
use crate::hex::hex_encode;
use crate::{b64, into_plain};
use aes::{
    Aes128, Aes256,
    cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7},
};
use cbc::cipher::crypto_common::rand_core::OsRng;
use fw_error::{FwError, FwResult};

pub struct AesCbc {
    key: Vec<u8>,
    iv: Vec<u8>,
    bits_type: AesBitsType,
}

impl AesCbc {
    pub fn new(
        key_str: &str,
        iv_str: &str,
        bits_type: AesBitsType,
        key_type: AesKeyDisplayType,
    ) -> FwResult<Self> {
        let key = key_type.to_bin(key_str)?;
        let iv = key_type.to_bin(iv_str)?;

        Ok(Self { key, iv, bits_type })
    }

    pub fn encrypt(&self, plains: &str) -> FwResult<String> {
        let encrypt_fn = match self.bits_type {
            AesBitsType::Bits256 => cbc_256_encrypt_floor,
            AesBitsType::Bits128 => cbc_128_encrypt_floor,
        };

        let ciphers = encrypt_fn(self.key.as_slice(), self.iv.as_slice(), plains.as_bytes())?;

        Ok(b64::encode(ciphers))
    }

    pub fn decrypt(&self, ciphers: &str) -> FwResult<String> {
        let ciphers = b64::decode(ciphers)?;

        let decrypt_fn = match self.bits_type {
            AesBitsType::Bits256 => cbc_256_decrypt_floor,
            AesBitsType::Bits128 => cbc_128_decrypt_floor,
        };

        let plains = decrypt_fn(self.key.as_slice(), self.iv.as_slice(), ciphers.as_slice())?;

        into_plain("AesCbc decrypt", plains)
    }
}

type Aes256CbcEnc = cbc::Encryptor<Aes256>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;
type Aes128CbcEnc = cbc::Encryptor<Aes128>;
type Aes128CbcDec = cbc::Decryptor<Aes128>;

/// AES-256-CBC 加密（底层字节版本）
pub fn cbc_256_encrypt_floor(key: &[u8], iv: &[u8], plains: &[u8]) -> FwResult<Vec<u8>> {
    let cipher = Aes256CbcEnc::new(key.into(), iv.into());
    let mut buffer = plains.to_vec();

    // 计算需要的空间（添加 PKCS7 填充）
    let block_size = 16;
    let padding_len = block_size - (buffer.len() % block_size);
    buffer.resize(buffer.len() + padding_len, 0);

    // 加密
    cipher
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, plains.len())
        .map_err(|e| FwError::CryptoError("cbc_256_encrypt", e.to_string()))?;

    Ok(buffer)
}

/// AES-256-CBC 加密（Base64 版本）
pub fn cbc_256_encrypt(key: &[u8], plains: &[u8]) -> FwResult<(String, String)> {
    let iv = Aes256CbcEnc::generate_iv(OsRng);

    let ciphertext = cbc_256_encrypt_floor(key, &iv, plains)?;

    Ok((b64::encode(&ciphertext), b64::encode(&iv)))
}

/// AES-256-CBC 解密（底层字节版本）
pub fn cbc_256_decrypt_floor(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> FwResult<Vec<u8>> {
    let cipher = Aes256CbcDec::new(key.into(), iv.into());
    let mut buffer = ciphertext.to_vec();

    // 解密并去除填充
    let decrypted = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|e| FwError::CryptoError("cbc_256_decrypt", e.to_string()))?;

    Ok(decrypted.to_vec())
}

/// AES-256-CBC 解密（Base64 版本）
pub fn cbc_256_decrypt(key: &[u8], ciphertext_b64: &str, iv_b64: &str) -> FwResult<String> {
    let ciphertext = b64::decode(ciphertext_b64)?;
    let iv = b64::decode(iv_b64)?;

    let plaintext = cbc_256_decrypt_floor(key, &iv, &ciphertext)?;

    into_plain("cbc_256_decrypt", plaintext)
}

/// AES-128-CBC 加密（底层字节版本）
pub fn cbc_128_encrypt_floor(key: &[u8], iv: &[u8], plains: &[u8]) -> FwResult<Vec<u8>> {
    let cipher = Aes128CbcEnc::new(key.into(), iv.into());
    let mut buffer = plains.to_vec();

    let block_size = 16;
    let padding_len = block_size - (buffer.len() % block_size);
    buffer.resize(buffer.len() + padding_len, 0);

    cipher
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, plains.len())
        .map_err(|e| FwError::CryptoError("cbc_128_encrypt", e.to_string()))?;

    Ok(buffer)
}

/// AES-128-CBC 加密（Base64 版本）
pub fn cbc_128_encrypt(key: &[u8], plains: &[u8]) -> FwResult<(String, String)> {
    let iv = Aes128CbcEnc::generate_iv(OsRng);

    let ciphertext = cbc_128_encrypt_floor(key, &iv, plains)?;

    Ok((b64::encode(&ciphertext), b64::encode(&iv)))
}

/// AES-128-CBC 解密（底层字节版本）
pub fn cbc_128_decrypt_floor(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> FwResult<Vec<u8>> {
    let cipher = Aes128CbcDec::new(key.into(), iv.into());
    let mut buffer = ciphertext.to_vec();

    let decrypted = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|e| FwError::CryptoError("cbc_128_decrypt", e.to_string()))?;

    Ok(decrypted.to_vec())
}

/// AES-128-CBC 解密（Base64 版本）
pub fn cbc_128_decrypt(key: &[u8], ciphertext_b64: &str, iv_b64: &str) -> FwResult<String> {
    let ciphertext = b64::decode(ciphertext_b64)?;
    let iv = b64::decode(iv_b64)?;

    let plaintext = cbc_128_decrypt_floor(key, &iv, &ciphertext)?;

    into_plain("cbc_128_decrypt", plaintext)
}

/// 生成随机 AES-256 密钥（十六进制）
pub fn gen_cbc_256_key_with_hex() -> String {
    let key = Aes256CbcEnc::generate_key(OsRng);
    hex_encode(key)
}

/// 生成随机 AES-128 密钥（十六进制）
pub fn gen_cbc_128_key_with_hex() -> String {
    let key = Aes128CbcEnc::generate_key(OsRng);
    hex_encode(key)
}

/// 生成随机 AES-256 密钥（十六进制）
pub fn gen_cbc_256_key_with_b64() -> String {
    let key = Aes256CbcEnc::generate_key(OsRng);
    b64::encode(key)
}

/// 生成随机 AES-128 密钥（十六进制）
pub fn gen_cbc_128_key_with_b64() -> String {
    let key = Aes128CbcEnc::generate_key(OsRng);
    b64::encode(key)
}

/// 生成随机 IV（16 字节）
pub fn gen_iv_with_hex() -> String {
    let iv = Aes256CbcEnc::generate_iv(OsRng);
    hex_encode(iv)
}

/// 生成随机 IV（16 字节）
pub fn gen_iv_with_b64() -> String {
    let iv = Aes256CbcEnc::generate_iv(OsRng);
    b64::encode(iv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::hex_decode;

    #[test]
    fn test_cbc_256() {
        let key_hex = gen_cbc_256_key_with_hex();
        let key = hex_decode(&key_hex).unwrap();

        let plaintext = "Hello AES-256-CBC！中文测试 🎉";
        println!("Original: {}", plaintext);

        // 加密
        let (cipher_b64, iv_b64) = cbc_256_encrypt(&key, plaintext.as_bytes()).unwrap();
        println!("Ciphertext (base64): {}...", &cipher_b64[..50]);
        println!("IV (base64): {}", iv_b64);

        // 解密
        let decrypted = cbc_256_decrypt(&key, &cipher_b64, &iv_b64).unwrap();
        println!("Decrypted: {}", decrypted);

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_cbc_128() {
        let key_hex = gen_cbc_128_key_with_hex();
        let key = hex_decode(&key_hex).unwrap();

        let plaintext = "Hello AES-128-CBC！测试数据";

        let (cipher_b64, iv_b64) = cbc_128_encrypt(&key, plaintext.as_bytes()).unwrap();
        let decrypted = cbc_128_decrypt(&key, &cipher_b64, &iv_b64).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_aes_cbc() {
        let key = gen_cbc_256_key_with_hex();
        let iv = gen_iv_with_hex();

        let ac = AesCbc::new(&key, &iv, AesBitsType::Bits256, AesKeyDisplayType::Hex).unwrap();

        let plaintext = "Hello AES-256-CBC！中文测试 🎉";
        println!("plaintext={}", plaintext);

        let ciphertext = ac.encrypt(plaintext).unwrap();
        println!("ciphertext={}", ciphertext);

        let plaintext = ac.decrypt(&ciphertext).unwrap();
        println!("plaintext={}", plaintext);
    }
}
