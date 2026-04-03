use crate::{b64, into_plain};
use fw_error::{FwError, FwResult};
use rand::thread_rng;
use rsa::pkcs8::{
    DecodePrivateKey, DecodePublicKey, Document, EncodePrivateKey, EncodePublicKey, LineEnding,
    SecretDocument,
};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};

pub struct Rsa {
    pri_key: RsaPrivateKey,
    pub_key: RsaPublicKey,
}

impl Rsa {
    pub fn from_str(pri_key: &str, pub_key: &str, display_type: RsaKeyDisplayType) -> FwResult<Self> {
        let pri_key = display_type.as_pri_key(pri_key)?;
        let pub_key = display_type.as_pub_key(pub_key)?;

        Ok(Self { pri_key, pub_key })
    }

    pub fn encrypt(&self, plains: &str) -> FwResult<String> {
        encrypt_floor(&self.pub_key, plains)
    }

    pub fn decrypt(&self, ciphers: &str) -> FwResult<String> {
        decrypt_floor(&self.pri_key, ciphers)
    }
}

pub enum RsaKeyDisplayType {
    B64,
    Pem,
}

impl RsaKeyDisplayType {
    pub fn as_pri_key(&self, pri_key: &str) -> FwResult<RsaPrivateKey> {
        match self {
            RsaKeyDisplayType::B64 => {
                let pri_der = b64::decode(pri_key)?;
                RsaPrivateKey::from_pkcs8_der(pri_der.as_slice())
                    .map_err(|e| FwError::CryptoError("rsa pri key from der", e.to_string()))
            }
            RsaKeyDisplayType::Pem => RsaPrivateKey::from_pkcs8_pem(pri_key)
                .map_err(|e| FwError::CryptoError("rsa pri key from pem", e.to_string())),
        }
    }

    pub fn as_pub_key(&self, pub_key: &str) -> FwResult<RsaPublicKey> {
        match self {
            RsaKeyDisplayType::B64 => {
                let pub_der = b64::decode(pub_key)?;
                RsaPublicKey::from_public_key_der(pub_der.as_slice())
                    .map_err(|e| FwError::CryptoError("rsa pub key from der", e.to_string()))
            }
            RsaKeyDisplayType::Pem => RsaPublicKey::from_public_key_pem(pub_key)
                .map_err(|e| FwError::CryptoError("rsa pub key from pem", e.to_string())),
        }
    }
}

pub fn encrypt_floor(pub_key: &RsaPublicKey, plains: &str) -> FwResult<String> {
    let mut rng = thread_rng();
    let values = pub_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, plains.as_bytes())
        .map_err(|e| FwError::CryptoError("rsa pub encrypt", e.to_string()))?;

    Ok(b64::encode(values))
}

pub fn encrypt(pub_key: &str, display_type: RsaKeyDisplayType, plains: &str) -> FwResult<String> {
    encrypt_floor(&display_type.as_pub_key(pub_key)?, plains)
}

pub fn decrypt_floor(pri_key: &RsaPrivateKey, ciphers: &str) -> FwResult<String> {
    let ciphers = b64::decode(ciphers)?;

    let values = pri_key
        .decrypt(Pkcs1v15Encrypt, ciphers.as_slice())
        .map_err(|e| FwError::CryptoError("rsa pri decrypt", e.to_string()))?;

    into_plain("rsa decrypt", values)
}

pub fn decrypt(pri_key: &str, display_type: RsaKeyDisplayType, ciphers: &str) -> FwResult<String> {
    decrypt_floor(&display_type.as_pri_key(pri_key)?, ciphers)
}

pub fn gen_rsa_key_pair_with_b64(bits: u16) -> FwResult<(String, String)> {
    let mut rng = thread_rng();
    let pri_key = RsaPrivateKey::new(&mut rng, bits as usize).unwrap();
    let pub_key = RsaPublicKey::from(&pri_key);
    let pri_pem = pri_key
        .to_pkcs8_der()
        .map_err(|e| FwError::CryptoError("rsa pri key to der", e.to_string()))?;
    let pub_pem = pub_key
        .to_public_key_der()
        .map_err(|e| FwError::CryptoError("rsa pub key to der", e.to_string()))?;

    Ok((
        b64::encode(pri_pem.as_bytes()),
        b64::encode(pub_pem.as_bytes()),
    ))
}

pub fn gen_rsa_key_pair_with_pem(bits: u16) -> FwResult<(String, String)> {
    let mut rng = thread_rng();
    let pri_key = RsaPrivateKey::new(&mut rng, bits as usize).unwrap();
    let pub_key = RsaPublicKey::from(&pri_key);

    let pri_pem = pri_key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|e| FwError::CryptoError("rsa pri key to pem", e.to_string()))?;
    let pub_pem = pub_key
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| FwError::CryptoError("rsa pub key to pem", e.to_string()))?;

    Ok((pri_pem.to_string(), pub_pem))
}

fn gen_rsa_key_pair(bits: u16) -> FwResult<(SecretDocument, Document)> {
    let mut rng = thread_rng();
    let pri_key = RsaPrivateKey::new(&mut rng, bits as usize).unwrap();
    let pub_key = RsaPublicKey::from(&pri_key);

    let pri_pem = pri_key
        .to_pkcs8_der()
        .map_err(|e| FwError::CryptoError("rsa pri key to der", e.to_string()))?;
    let pub_pem = pub_key
        .to_public_key_der()
        .map_err(|e| FwError::CryptoError("rsa pub key to der", e.to_string()))?;

    Ok((pri_pem, pub_pem))
}

#[cfg(test)]
mod tests {
    use crate::rsa::{
        Rsa, RsaKeyDisplayType, decrypt, encrypt, gen_rsa_key_pair_with_b64,
        gen_rsa_key_pair_with_pem,
    };

    #[test]
    fn test_gen_key() {
        let (pri_key, pub_key) = gen_rsa_key_pair_with_b64(2048).unwrap();
        println!("pri_key={pri_key}");
        println!("pub_key={pub_key}");

        let (pri_key, pub_key) = gen_rsa_key_pair_with_pem(2048).unwrap();
        println!("pri_key={pri_key}");
        println!("pub_key={pub_key}");
    }

    #[test]
    fn test_rsa_encrypt_decrypt() {
        let (pri_pem, pub_pem) = gen_rsa_key_pair_with_pem(2048).unwrap();
        let plain = "我是大哥啊@fdfdsf_*&^$#iewj~fdlsjl---------~33!- - ~ -~@!@!!!!!!fdsf火星文誃尐亽籟萿亍丗，萿嘚像嗰怎庅說嘟卟嗵の徣ロ";

        let cipher = encrypt(&pub_pem, RsaKeyDisplayType::Pem, plain).unwrap();
        println!("cipher={}", cipher);

        let plain = decrypt(&pri_pem, RsaKeyDisplayType::Pem, &cipher).unwrap();

        println!("plain={}", plain);

        let (pri_pem, pub_pem) = gen_rsa_key_pair_with_b64(2048).unwrap();
        let plain = "我是大哥啊@fdfdsf_*&^$#iewj~fdlsjl---------~33!- - ~ -~@!@!!!!!!fdsf火星文誃尐亽籟萿亍丗，萿嘚像嗰怎庅說嘟卟嗵の徣ロ";

        let cipher = encrypt(&pub_pem, RsaKeyDisplayType::B64, plain).unwrap();
        println!("cipher={}", cipher);

        let plain = decrypt(&pri_pem, RsaKeyDisplayType::B64, &cipher).unwrap();

        println!("plain={}", plain);
    }

    #[test]
    fn test_rsa_struct() {
        let plain = "我是大哥啊@fdfdsf_*&^$#iewj~fdlsjl---------~33!- - ~ -~@!@!!!!!!fdsf火星文誃尐亽籟萿亍丗，萿嘚像嗰怎庅說嘟卟嗵の徣ロ";

        let (pri_pem, pub_pem) = gen_rsa_key_pair_with_pem(2048).unwrap();
        let rsa = Rsa::from_str(&pri_pem, &pub_pem, RsaKeyDisplayType::Pem).unwrap();
        let cipher = rsa.encrypt(plain).unwrap();
        println!("cipher={}", cipher);
        let plain = rsa.decrypt(&cipher).unwrap();
        println!("plain={}", plain);

        let (pri_pem, pub_pem) = gen_rsa_key_pair_with_b64(4096).unwrap();
        let rsa = Rsa::from_str(&pri_pem, &pub_pem, RsaKeyDisplayType::B64).unwrap();
        let cipher = rsa.encrypt(&plain).unwrap();
        println!("cipher={}", cipher);
        let plain = rsa.decrypt(&cipher).unwrap();
        println!("plain={}", plain);
    }
}
