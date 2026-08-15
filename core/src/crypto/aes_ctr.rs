use aes::{Aes256, cipher::{KeyIvInit, StreamCipher}};
use ctr::Ctr128BE;
use generic_array::GenericArray;
use hkdf::Hkdf;
use rand::RngCore;
use rand_chacha::ChaCha20Rng;
use rand::SeedableRng;
use sha2::Sha256;
use zeroize::Zeroize;

#[derive(Clone)]
pub struct AesCtrCipher {
    key: [u8; 32],
}

impl Zeroize for AesCtrCipher {
    fn zeroize(&mut self) {
        self.key.zeroize();
    }
}

impl Drop for AesCtrCipher {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl AesCtrCipher {
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    pub fn derive_key(seed: &[u8], salt: &[u8], info: &[u8]) -> Self {
        let hkdf = Hkdf::<Sha256>::new(Some(salt), seed);
        let mut key = [0u8; 32];
        hkdf.expand(info, &mut key).expect("HKDF expand failed");
        Self { key }
    }

    pub fn encrypt(&self, plaintext: &[u8], nonce: &[u8; 16]) -> Vec<u8> {
        let nonce_ga = GenericArray::from_slice(nonce);
        let mut ctr = Ctr128BE::<Aes256>::new(&self.key.into(), nonce_ga);
        let mut buf = plaintext.to_vec();
        ctr.apply_keystream(&mut buf);
        buf
    }

    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8; 16]) -> Vec<u8> {
        let nonce_ga = GenericArray::from_slice(nonce);
        let mut ctr = Ctr128BE::<Aes256>::new(&self.key.into(), nonce_ga);
        let mut buf = ciphertext.to_vec();
        ctr.apply_keystream(&mut buf);
        buf
    }

    pub fn key_bytes(&self) -> &[u8; 32] { &self.key }

    pub fn random_nonce() -> [u8; 16] {
        let mut nonce = [0u8; 16];
        ChaCha20Rng::from_entropy().fill_bytes(&mut nonce);
        nonce
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_ctr_kat_encrypt_decrypt() {
        let key = [0x00; 32];
        let cipher = AesCtrCipher { key };
        let nonce = [0x01; 16];
        let plaintext = b"ChaosSeal protocol engine test vector";
        let ct = cipher.encrypt(plaintext, &nonce);
        let pt = cipher.decrypt(&ct, &nonce);
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_aes_ctr_kat_rfc3686_pattern_1() {
        let key = [
            0xAE, 0x68, 0x52, 0xF8, 0x12, 0x10, 0x67, 0xCC,
            0x4B, 0xF7, 0xA5, 0x76, 0x55, 0x77, 0xF3, 0x9E,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let cipher = AesCtrCipher::new(key);
        let nonce = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        let plaintext = [
            0x6B, 0xC1, 0xBE, 0xE2, 0x2E, 0x40, 0x9F, 0x96,
            0xE9, 0x3D, 0x7E, 0x11, 0x73, 0x93, 0x17, 0x2A,
        ];
        let expected_ciphertext = [
            0xC4, 0x38, 0x8F, 0x30, 0x9C, 0xF1, 0xEF, 0x44,
            0x22, 0xC0, 0x66, 0x09, 0xF0, 0x1F, 0xD4, 0xA8,
        ];
        let ct = cipher.encrypt(&plaintext, &nonce);
        assert_eq!(ct, expected_ciphertext, "AES-CTR RFC 3686 pattern 1 mismatch");
        let pt = cipher.decrypt(&ct, &nonce);
        assert_eq!(pt, plaintext);
    }
}
