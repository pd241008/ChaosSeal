use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use hkdf::Hkdf;
use rand::RngCore;
use rand_chacha::ChaCha20Rng;
use rand::SeedableRng;
use sha2::Sha256;
use zeroize::Zeroize;

#[derive(Clone)]
pub struct AesGcmCipher {
    key: [u8; 32],
}

impl Zeroize for AesGcmCipher {
    fn zeroize(&mut self) {
        self.key.zeroize();
    }
}

impl Drop for AesGcmCipher {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl AesGcmCipher {
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    pub fn derive_key(seed: &[u8], salt: &[u8], info: &[u8]) -> Self {
        let hkdf = Hkdf::<Sha256>::new(Some(salt), seed);
        let mut key = [0u8; 32];
        hkdf.expand(info, &mut key).expect("HKDF expand failed");
        Self { key }
    }

    pub fn encrypt(&self, plaintext: &[u8], nonce_bytes: &[u8; 12]) -> Vec<u8> {
        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);
        cipher.encrypt(nonce, plaintext).expect("encryption failure!")
    }

    pub fn decrypt(&self, ciphertext: &[u8], nonce_bytes: &[u8; 12]) -> Vec<u8> {
        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);
        cipher.decrypt(nonce, ciphertext).expect("decryption failure!")
    }

    pub fn key_bytes(&self) -> &[u8; 32] { &self.key }

    pub fn random_nonce() -> [u8; 12] {
        let mut nonce = [0u8; 12];
        ChaCha20Rng::from_entropy().fill_bytes(&mut nonce);
        nonce
    }
}
