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
    crate::crypto::hmac_sha256::compute(packet_key, ciphertext)
}

/// Verify the HMAC commitment.
pub fn verify_hmac(packet_key: &[u8; 32], ciphertext: &[u8], expected: &[u8; 32]) -> bool {
    crate::crypto::hmac_sha256::verify(packet_key, ciphertext, expected)
}

/// Flip a single bit in a byte slice at the given bit position.
pub fn flip_bit(data: &mut [u8], bit_pos: usize) {
    let byte_idx = bit_pos / 8;
    let bit_idx = bit_pos % 8;
    if byte_idx < data.len() {
        data[byte_idx] ^= 1 << bit_idx;
    }
}

/// Measure how many epochs elapse before a single-bit corruption in the
/// session seed is detected by HMAC verification.  This is the core
/// corruption-sensitivity test comparing counter-mode vs chaotic-pendulum.
///
/// Returns (epochs_until_detection, total_tested).
pub fn measure_corruption_detection(
    corrupted_seed: &[u8],
    original_seed: &[u8],
    salt: &[u8],
    info: &[u8],
    packets_per_epoch: u32,
    max_epochs: usize,
) -> (usize, usize) {
    use crate::crypto::AesGcmCipher;

    for epoch in 0..max_epochs {
        let mut deriver = CounterKeyDeriver::new(corrupted_seed, salt, info);

        for p in 0..packets_per_epoch {
            let (key, _c) = deriver.next_key();
            let cipher = AesGcmCipher::new(key);

            // Encrypt a fixed payload
            let payload = vec![0xABu8; 1024];
            let nonce = [0u8; 12]; // deterministic for reproducibility
            let ciphertext = cipher.encrypt(&payload, &nonce);

            // Compute HMAC with the corrupted key
            let tag = hmac_commitment(&key, &ciphertext);

            // "Verify" using the original (uncorrupted) key's expectation
            let orig_key = derive_packet_key(original_seed, salt, info, p);
            let expected_tag = hmac_commitment(&orig_key, &ciphertext);

            if tag != expected_tag {
                return (epoch, max_epochs);
            }
            // Also check ciphertext differs (indirect detection)
            let orig_cipher = AesGcmCipher::new(orig_key);
            let orig_ct = orig_cipher.encrypt(&payload, &nonce);
            if ciphertext != orig_ct {
                return (epoch, max_epochs);
            }
            // The only way these are equal is if the key derivation is the same
            // — which it won't be if the seed was actually corrupted.
            // In practice: different seeds → different keys → different HMAC.
        }
    }
    (max_epochs, max_epochs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_derivation_deterministic() {
        let seed = [0xAA; 32];
        let salt = b"test-salt";
        let info = b"test-info";

        let k1 = derive_packet_key(&seed, salt, info, 0);
        let k2 = derive_packet_key(&seed, salt, info, 0);
        assert_eq!(k1, k2);

        let k3 = derive_packet_key(&seed, salt, info, 1);
        assert_ne!(k1, k3);
    }

    #[test]
    fn test_flip_bit() {
        let mut data = [0x00u8; 4];
        flip_bit(&mut data, 0);  // bit 0 of byte 0
        assert_eq!(data[0], 0x01);
        flip_bit(&mut data, 7);  // bit 7 of byte 0
        assert_eq!(data[0], 0x81);
        flip_bit(&mut data, 8);  // bit 0 of byte 1
        assert_eq!(data[1], 0x01);
    }
}
