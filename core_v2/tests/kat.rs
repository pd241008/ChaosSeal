use chaosseal_core::*;

#[test]
fn test_kat_aes256_gcm_cross_validated() {
    let key = [
        0xFE, 0xFF, 0xE9, 0x92, 0x86, 0x65, 0x73, 0x1C,
        0x6D, 0x6A, 0x8F, 0x94, 0x67, 0x30, 0x83, 0x08,
        0xFE, 0xFF, 0xE9, 0x92, 0x86, 0x65, 0x73, 0x1C,
        0x6D, 0x6A, 0x8F, 0x94, 0x67, 0x30, 0x83, 0x08,
    ];
    let cipher = AesGcmCipher::new(key);
    let nonce = [0xCA, 0xFE, 0xBA, 0xBE, 0xFA, 0xCE, 0xDA, 0xDB, 0xAD, 0xEC, 0xAF, 0x88];
    let plaintext = [
        0xD9, 0x31, 0x32, 0x25, 0xF8, 0x84, 0x06, 0xE5,
        0xA5, 0x59, 0x09, 0xC5, 0xAF, 0xF5, 0x26, 0x9A,
        0x86, 0xA7, 0xA9, 0x53, 0x15, 0x34, 0xF7, 0xDA,
        0x2E, 0x4C, 0x30, 0x3D, 0x8A, 0x31, 0x8A, 0x72,
        0x1C, 0x3C, 0x0C, 0x95, 0x95, 0x68, 0x09, 0x53,
        0x2F, 0xCF, 0x0E, 0x24, 0x49, 0xA6, 0xB5, 0x25,
        0xB1, 0x6A, 0xED, 0xF5, 0xAA, 0x0D, 0xE6, 0x57,
        0xBA, 0x63, 0x7B, 0x39,
    ];
    let expected = [
        0x2C, 0xBE, 0xEA, 0x33, 0x09, 0xA5, 0x84, 0x0B,
        0xC2, 0xE6, 0x38, 0x77, 0x31, 0x09, 0xAC, 0x05,
        0x58, 0x6A, 0x7D, 0xF2, 0x69, 0xDD, 0x89, 0x02,
        0x28, 0x64, 0x59, 0x5F, 0x44, 0x32, 0x34, 0x98,
        0x30, 0x4B, 0xB6, 0xDD, 0x8B, 0x68, 0x73, 0x7C,
        0x86, 0x80, 0x91, 0xBF, 0xE9, 0x5D, 0x7C, 0x15,
        0x49, 0xD0, 0x82, 0x19, 0xE2, 0x1D, 0x3B, 0xD0,
        0xFA, 0xE0, 0x33, 0x60, 0x79, 0xFA, 0xB9, 0xAD,
        0xCB, 0xDF, 0x09, 0xA6, 0x25, 0xD2, 0x72, 0xFE,
        0x4D, 0x15, 0x78, 0x61,
    ];
    let ct = cipher.encrypt(&plaintext, &nonce);
    assert_eq!(ct, expected, "AES-256-GCM ciphertext+tag mismatch");
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
    let l1 = estimator.estimate(&|t, s| pendulum.derivatives(t, s), &|s| pendulum.jacobian(s), &|s| pendulum.apply_reinjection(s), Q32_32::ZERO, &state);
    let l1_again = estimator.estimate(&|t, s| pendulum.derivatives(t, s), &|s| pendulum.jacobian(s), &|s| pendulum.apply_reinjection(s), Q32_32::ZERO, &state);
    assert_eq!(l1.to_bits(), l1_again.to_bits(), "Lyapunov estimator not deterministic");
}

fn finite_difference_jacobian(pendulum: &MultiPendulum, state: &[Q32_32], delta: f64) -> Vec<Vec<f64>> {
    let dim = state.len();
    let mut jac = vec![vec![0f64; dim]; dim];
    let dq = Q32_32::from_f64(delta);
    for c in 0..dim {
        let mut sp = state.to_vec();
        let mut sm = state.to_vec();
        sp[c] = sp[c] + dq;
        sm[c] = sm[c] - dq;
        let fp = pendulum.derivatives(Q32_32::ZERO, &sp);
        let fm = pendulum.derivatives(Q32_32::ZERO, &sm);
        for r in 0..dim {
            jac[r][c] = (fp[r].to_f64() - fm[r].to_f64()) / (2.0 * delta);
        }
    }
    jac
}

#[test]
fn test_jacobian_matches_finite_difference() {
    let pendulum = MultiPendulum::new(3, Q32_32::from_f64(1.0), Q32_32::from_f64(1.0), Q32_32::from_f64(0.1), Q32_32::from_f64(0.5));
    let state: Vec<Q32_32> = [0.5, -0.7, 0.3, 0.9, -0.4, 0.2]
        .iter().map(|&v| Q32_32::from_f64(v)).collect();
    let analytic = pendulum.jacobian(&state);
    let fd = finite_difference_jacobian(&pendulum, &state, 1e-3);
    let mut worst = 0f64;
    let mut worst_rc = (0usize, 0usize);
    for r in 0..state.len() {
        for c in 0..state.len() {
            let a = analytic[r][c].to_f64();
            let d = fd[r][c];
            let err = (a - d).abs();
            let scale = a.abs().max(1e-6);
            if err / scale > worst {
                worst = err / scale;
                worst_rc = (r, c);
            }
            assert!(err <= scale * 0.02 + 1e-4,
                "jacobian[{r}][{c}]: analytic {a:.6} vs finite-diff {d:.6} (rel {:.4})",
                err / scale);
        }
    }
    eprintln!("jacobian finite-difference check passed; worst rel error {worst:.4} at {worst_rc:?}");
}

#[test]
fn test_tangent_product_identity() {
    // J(x)v must equal the directional derivative (f(x+eps v) - f(x))/eps:
    // the core identity the Benettin tangent update relies on.
    let pendulum = MultiPendulum::new(3, Q32_32::from_f64(1.0), Q32_32::from_f64(1.0), Q32_32::from_f64(0.1), Q32_32::from_f64(0.5));
    let state: Vec<Q32_32> = [0.5, -0.7, 0.3, 0.9, -0.4, 0.2]
        .iter().map(|&v| Q32_32::from_f64(v)).collect();
    let v: Vec<f64> = [0.1, -0.5, 0.8, 0.3, -0.2, 0.6].to_vec();
    let jac = pendulum.jacobian(&state);
    let mut jv = vec![0f64; state.len()];
    for r in 0..state.len() {
        for c in 0..state.len() {
            jv[r] += jac[r][c].to_f64() * v[c];
        }
    }
    let eps = 1e-3;
    let mut sp = state.to_vec();
    let mut sm = state.to_vec();
    for c in 0..state.len() {
        sp[c] = sp[c] + Q32_32::from_f64(eps * v[c]);
        sm[c] = sm[c] - Q32_32::from_f64(eps * v[c]);
    }
    let fp = pendulum.derivatives(Q32_32::ZERO, &sp);
    let fm = pendulum.derivatives(Q32_32::ZERO, &sm);
    for r in 0..state.len() {
        let fd_directional = (fp[r].to_f64() - fm[r].to_f64()) / (2.0 * eps);
        assert!((jv[r] - fd_directional).abs() <= jv[r].abs().max(1e-6) * 0.02 + 1e-4,
            "J*v[{r}]: {:.6} vs directional FD {:.6}", jv[r], fd_directional);
    }
    eprintln!("tangent product identity Jv = (f(x+ev)-f(x-ev))/2e verified");
}

#[test]
fn test_bee_ciphertext_size_scaling() {
    let sizes: Vec<usize> = (1..=16).map(|r| BEEEngine::new(1024, r).ciphertext_size_min()).collect();
    for (i, &size) in sizes.iter().enumerate() {
        assert!(size > 0, "Ciphertext size must be positive for r={}", i + 1);
        assert!(size < 100000, "Ciphertext size must be reasonable for r={}", i + 1);
    }
}
