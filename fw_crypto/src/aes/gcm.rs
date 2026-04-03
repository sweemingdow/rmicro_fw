use crate::b64;
use crate::hex::hex_encode;
use aes_gcm::{
    AeadCore, Aes128Gcm, Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng},
};
use fw_error::{FwError, FwResult};

pub fn gcm_256_encrypt_floor(key: &[u8], plains: &[u8]) -> FwResult<(Vec<u8>, Vec<u8>)> {
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphers = cipher
        .encrypt(&nonce, plains)
        .map_err(|e| FwError::CryptoError("gcm_256_encrypt", e.to_string()))?;

    Ok((ciphers, nonce.to_vec()))
}

pub fn gcm_256_encrypt(key: &[u8], plains: &[u8]) -> FwResult<(String, String)> {
    let (ciphers, nonce) = gcm_256_encrypt_floor(key, plains)?;

    Ok((b64::encode(&ciphers), b64::encode(nonce)))
}

pub fn gcm_256_decrypt_floor(key: &[u8], ciphers: &[u8], nonce: &[u8]) -> FwResult<Vec<u8>> {
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);

    let nonce = Nonce::from_slice(nonce);

    let plains = cipher
        .decrypt(nonce, ciphers.as_ref())
        .map_err(|e| FwError::CryptoError("gcm_256_decrypt", e.to_string()))?;

    Ok(plains)
}

pub fn gcm_256_decrypt(key: &[u8], ciphers: &str, nonce: &str) -> FwResult<String> {
    let ciphers = b64::decode(ciphers)?;
    let nonce = b64::decode(nonce)?;

    let plains = gcm_256_decrypt_floor(key, ciphers.as_slice(), nonce.as_slice())?;

    Ok(String::from_utf8(plains)
        .map_err(|e| FwError::CryptoError("gcm_256_decrypt", e.to_string()))?)
}

// 32个字符
pub fn gen_gcm_256_key_with_hex() -> String {
    let key = Aes256Gcm::generate_key(OsRng);
    hex_encode(key)
}

// 16个字符
pub fn gen_gcm_128_key_with_hex() -> String {
    let key = Aes128Gcm::generate_key(OsRng);
    hex_encode(key)
}

// 32个字符
pub fn gen_gcm_256_key_with_b64() -> String {
    let key = Aes256Gcm::generate_key(OsRng);
    b64::encode(key)
}

// 16个字符
pub fn gen_gcm_128_key_with_b64() -> String {
    let key = Aes128Gcm::generate_key(OsRng);
    b64::encode(key)
}

#[cfg(test)]
mod tests {
    use crate::aes::gcm::{
        gcm_256_decrypt, gcm_256_encrypt, gen_gcm_128_key_with_hex, gen_gcm_256_key_with_hex,
    };
    use crate::hex::{hex_decode, hex_encode};
    use aes_gcm::aead::OsRng;
    use aes_gcm::{Aes128Gcm, Aes256Gcm, KeyInit};

    #[test]
    fn test_gen_aes_key() {
        let key = gen_gcm_256_key_with_hex();
        println!("key={}", hex_encode(key));

        // 5f3ebc7fb529ae64f3ef742d2fbf1694c29b5ca62ba6933052e169bf17a56c3a
        let key = Aes256Gcm::generate_key(OsRng);
        println!("key={}", hex_encode(key));

        let key = gen_gcm_128_key_with_hex();
        let key = hex_encode(key);
        println!("key={}", key);

        let key = hex_decode(key);
        println!("{key:?}");

        // 5f3ebc7fb529ae64f3ef742d2fbf1694c29b5ca62ba6933052e169bf17a56c3a
        let key = Aes128Gcm::generate_key(OsRng);
        println!("key={}", hex_encode(key));
    }

    #[test]
    fn test_aes_gcm() {
        let key = gen_gcm_256_key_with_hex();
        println!("key={}", key);

        let key = hex_decode(key).unwrap();

        let plain = "我是大哥啊@fdfdsf_*&^$#iewj~fdlsjl---------~33!- - ~ -~@!@!!!!!!fdsf火星文誃尐亽籟萿亍丗，萿嘚像嗰怎庅說嘟卟嗵の徣ロ";
        println!("plain={}", plain);

        let (cipher, nonce) = gcm_256_encrypt(&key, plain.as_bytes()).unwrap();
        println!("cipher={}, nonce={}", cipher, nonce);

        let plain = gcm_256_decrypt(&key, &cipher, &nonce).unwrap();
        println!("plain={}", plain);
    }
}
