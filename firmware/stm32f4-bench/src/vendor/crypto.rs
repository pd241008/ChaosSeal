//! ChaosSeal crypto primitives (AES-256-GCM, HKDF counter deriver, HMAC-SHA256).
//!
//! Vendored verbatim from `core_v2/src/crypto/{aes_gcm,counter,hmac_sha256}.rs`
//! at core_v2 @ 29be3b5 (merge of PR #20). Deltas (no behavioral changes to
//! the benchmarked paths):
//! - cross-module `crate::crypto::` paths -> `crate::vendor::crypto::`
//! - `AesGcmCipher::random_nonce()` removed: it needs OS entropy (`rand`
//!   `from_entropy`, std); the benchmark uses deterministic nonces exactly
//!   like the netsim corruption-test protocol does.
//! - upstream `#[cfg(test)]` modules trimmed (host-only)

#![allow(dead_code)]

use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroize;

pub mod hmac_sha256 {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    pub type HmacSha256 = Hmac<Sha256>;

    pub fn compute(key: &[u8], message: &[u8]) -> [u8; 32] {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key valid");
        mac.update(message);
        let result = mac.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result.into_bytes());
        out
    }

    pub fn verify(key: &[u8], message: &[u8], expected: &[u8; 32]) -> bool {
        let computed = compute(key, message);
        computed == *expected
    }
}

pub mod aes_gcm {
    // `::aes_gcm` = the external crate (the bare name would resolve to this
    // nested module and shadow it under 2018+ path rules).
    use ::aes_gcm::aead::{Aead, KeyInit};
    use ::aes_gcm::{Aes256Gcm, Key, Nonce};
    use alloc::vec::Vec;
    use super::{Hkdf, Sha256, Zeroize};

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
    }
}

pub mod counter {
    use alloc::{vec, vec::Vec};
    use hkdf::Hkdf;
    use sha2::Sha256;
    use zeroize::Zeroize;

    /// Counter-mode key derivation: K_i = HKDF-Expand(SessionSeed || i, info, 32).
    ///
    /// This is the deterministic baseline against which the chaotic pendulum is
    /// compared.  The session seed is fixed per epoch; the counter `i` increments
    /// per packet.  No Lyapunov dynamics are involved — key diversity comes solely
    /// from the HKDF counter input.
    #[derive(Clone)]
    pub struct CounterKeyDeriver {
        session_seed: Vec<u8>,
        salt: Vec<u8>,
        info: Vec<u8>,
        counter: u32,
    }

    impl Zeroize for CounterKeyDeriver {
        fn zeroize(&mut self) {
            self.session_seed.zeroize();
            self.counter = 0;
        }
    }

    impl Drop for CounterKeyDeriver {
        fn drop(&mut self) {
            self.session_seed.zeroize();
        }
    }

    impl CounterKeyDeriver {
        pub fn new(session_seed: &[u8], salt: &[u8], info: &[u8]) -> Self {
            Self {
                session_seed: session_seed.to_vec(),
                salt: salt.to_vec(),
                info: info.to_vec(),
                counter: 0,
            }
        }

        /// Derive the next packet key.  Returns (key, counter_value).
        pub fn next_key(&mut self) -> ([u8; 32], u32) {
            let mut ikm = self.session_seed.clone();
            ikm.extend_from_slice(&self.counter.to_be_bytes());

            let hkdf = Hkdf::<Sha256>::new(Some(&self.salt), &ikm);
            let mut key = [0u8; 32];
            hkdf.expand(&self.info, &mut key).expect("HKDF expand failed");

            let c = self.counter;
            self.counter = 1;
            (key, c)
        }

        /// Reset counter (called at epoch boundary in the protocol).
        pub fn reset(&mut self) {
            self.counter = 0;
        }

        pub fn current_counter(&self) -> u32 {
            self.counter
        }

        /// Derive a session seed from the same root the pendulum path uses,
        /// so both protocols share a common trust anchor.
        pub fn session_seed_for_epoch(epoch: u64) -> [u8; 32] {
            let mut ikm = vec![0u8; 8 + 43];
            ikm[..8].copy_from_slice(&epoch.to_be_bytes());
            let root = b"counter-mode-root-epoch-seed-for-chaosseal";
            ikm[8..8 + root.len()].copy_from_slice(root);
            let hkdf = Hkdf::<Sha256>::new(Some(b"chaosseal-session-seed"), &ikm);
            let mut out = [0u8; 32];
            hkdf.expand(b"epoch-seed-derivation", &mut out)
                .expect("HKDF expand failed");
            out
        }
    }
}

/// Derive the AES-256 key for a given counter value (stateless convenience).
pub fn derive_packet_key(session_seed: &[u8], salt: &[u8], info: &[u8], counter: u32) -> [u8; 32] {
    let mut ikm = session_seed.to_vec();
    ikm.extend_from_slice(&counter.to_be_bytes());
    let hkdf = Hkdf::<Sha256>::new(Some(salt), &ikm);
    let mut key = [0u8; 32];
    hkdf.expand(info, &mut key).expect("HKDF expand failed");
    key
}

/// Compute the HMAC of a ciphertext using a packet key (same as chaosseal).
pub fn hmac_commitment(packet_key: &[u8; 32], ciphertext: &[u8]) -> [u8; 32] {
    crate::vendor::crypto::hmac_sha256::compute(packet_key, ciphertext)
}

/// Verify the HMAC commitment.
pub fn verify_hmac(packet_key: &[u8; 32], ciphertext: &[u8], expected: &[u8; 32]) -> bool {
    crate::vendor::crypto::hmac_sha256::verify(packet_key, ciphertext, expected)
}

/// Flip a single bit in a byte slice at the given bit position.
pub fn flip_bit(data: &mut [u8], bit_pos: usize) {
    let byte_idx = bit_pos / 8;
    let bit_idx = bit_pos % 8;
    if byte_idx < data.len() {
        data[byte_idx] ^= 1 << bit_idx;
    }
}
