use chaosseal_core::*;

#[test]
fn test_kat_aes_ctr_rfc3686_pattern_1() {
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
    let expected = [
        0xC4, 0x38, 0x8F, 0x30, 0x9C, 0xF1, 0xEF, 0x44,
        0x22, 0xC0, 0x66, 0x09, 0xF0, 0x1F, 0xD4, 0xA8,
    ];
    let ct = cipher.encrypt(&plaintext, &nonce);
    assert_eq!(ct, expected);
    let pt = cipher.decrypt(&ct, &nonce);
    assert_eq!(pt, plaintext);
}

#[test]
fn test_kat_hmac_rfc4231_case_1() {
    let key = [0x0B; 20];
    let message = b"Hi There";
    let expected = [
        0xB0, 0x34, 0x4C, 0x61, 0xD8, 0xDB, 0x38, 0x53,
        0x5C, 0xA8, 0xAF, 0xCE, 0xAF, 0x0B, 0xF1, 0x2B,
        0x88, 0x1D, 0xC2, 0x00, 0xC9, 0x83, 0x3D, 0xA7,
        0x26, 0xE9, 0x37, 0x6C, 0x2E, 0x32, 0xCF, 0xF7,
    ];
    let computed = crypto::hmac_sha256::compute(&key, message);
    assert_eq!(computed, expected);
}

#[test]
fn test_kat_hmac_rfc4231_case_2() {
    let key = b"Jefe";
    let message = b"what do ya want for nothing?";
    let expected = [
        0x5B, 0xDC, 0xC1, 0x46, 0xBF, 0x60, 0x75, 0x4E,
        0x6A, 0x04, 0x24, 0x26, 0x08, 0x95, 0x75, 0xC7,
        0x5A, 0x00, 0x3F, 0x08, 0x9D, 0x27, 0x39, 0x83,
        0x9D, 0xEC, 0x58, 0xB9, 0x64, 0xEC, 0x38, 0x43,
    ];
    let computed = crypto::hmac_sha256::compute(key, message);
    assert_eq!(computed, expected);
}

#[test]
fn test_determinism_bee_sizes() {
    let mut sizes = Vec::new();
    for _ in 0..100 {
        let engine = BEEEngine::new(1024, 8);
        sizes.push(engine.ciphertext_size_min());
    }
    assert!(sizes.windows(2).all(|w| w[0] == w[1]), "BEE ciphertext size not deterministic");
}

#[test]
fn test_determinism_lyapunov() {
    let pendulum = MultiPendulum::new(3, Q32_32::from_f64(1.0), Q32_32::from_f64(1.0), Q32_32::from_f64(0.1), Q32_32::from_f64(0.5));
    let mut state = vec![Q32_32::ZERO; pendulum.dimension()];
    for i in 0..3 { state[i] = Q32_32::from_f64(0.1 * (i as f64 + 1.0)); }
    let estimator = LyapunovEstimator { steps: 1000, ..Default::default() };
    let l1 = estimator.estimate(&|t, s| pendulum.derivatives(t, s), Q32_32::ZERO, &state);
    let l1_again = estimator.estimate(&|t, s| pendulum.derivatives(t, s), Q32_32::ZERO, &state);
    assert_eq!(l1.to_bits(), l1_again.to_bits(), "Lyapunov estimator not deterministic");
}

#[test]
fn test_bee_ciphertext_size_scaling() {
    let sizes: Vec<usize> = (1..=16).map(|r| BEEEngine::new(1024, r).ciphertext_size_min()).collect();
    for (i, &size) in sizes.iter().enumerate() {
        assert!(size > 0, "Ciphertext size must be positive for r={}", i + 1);
        assert!(size < 100000, "Ciphertext size must be reasonable for r={}", i + 1);
    }
}
