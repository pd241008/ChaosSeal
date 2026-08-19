use crate::{BEEEngine, LyapunovEstimator, MultiPendulum};
use crate::fixed::Q32_32;

use std::ffi::{CString};
use std::os::raw::c_char;

#[repr(C)]
pub struct CChaosSealResult {
    pub code: i32,
    pub message: *mut c_char,
    pub lambda1: f64,
    pub ciphertext_size: usize,
    pub dt_bound: f64,
}

#[no_mangle]
pub extern "C" fn chaosseal_compute_lambda1(
    pendulum_count: usize,
    mass: f64,
    length: f64,
    damping: f64,
    coupling: f64,
    steps: usize,
) -> CChaosSealResult {
    let pendulum = MultiPendulum::new(
        pendulum_count,
        Q32_32::from_f64(mass),
        Q32_32::from_f64(length),
        Q32_32::from_f64(damping),
        Q32_32::from_f64(coupling),
    );
    let mut state = vec![Q32_32::ZERO; pendulum.dimension()];
    for i in 0..pendulum_count {
        state[i] = Q32_32::from_f64(0.1 * (i as f64 + 1.0));
    }
    let estimator = LyapunovEstimator { steps, ..Default::default() };
    let lambda1 = estimator.estimate(&|t, s| pendulum.derivatives(t, s), &|s| pendulum.apply_reinjection(s), Q32_32::ZERO, &state);

    let cmsg = CString::new("ok").unwrap();
    CChaosSealResult {
        code: 0,
        message: cmsg.into_raw(),
        lambda1: lambda1.to_f64(),
        ciphertext_size: 0,
        dt_bound: f64::max(256.0 * std::f64::consts::LN_2 / lambda1.to_f64(), 1.0 / lambda1.to_f64()),
    }
}

#[no_mangle]
pub extern "C" fn chaosseal_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)); }
    }
}

#[no_mangle]
pub extern "C" fn chaosseal_bee_ciphertext_size(n: usize, r: usize) -> usize {
    let engine = BEEEngine::new(n, r);
    engine.ciphertext_size_min()
}

#[repr(C)]
pub struct CEpochCryptoResult {
    pub crypto_overhead: usize,
}

#[no_mangle]
pub extern "C" fn chaosseal_epoch_keygen() {
    // Wire real physics engine: compute the 1200s epoch chaotic evolution
    let pendulum = MultiPendulum::new(
        3,
        Q32_32::from_f64(1.0),
        Q32_32::from_f64(1.0),
        Q32_32::from_f64(0.1),
        Q32_32::from_f64(0.5),
    );
    let mut state = vec![Q32_32::ZERO; pendulum.dimension()];
    for i in 0..3 {
        state[i] = Q32_32::from_f64(3.0); // high energy chaotic initial state
    }
    
    let integrator = crate::kinematics::Rk4Integrator::new(Q32_32::from_f64(0.01));
    let mut t = Q32_32::ZERO;
    // 1200s epoch at dt=0.01s = 120,000 steps
    for _ in 0..120000 {
        pendulum.apply_reinjection(&mut state);
        let (nt, ns) = integrator.step(&|t, s| pendulum.derivatives(t, s), t, &state);
        t = nt;
        state = ns;
    }
}

#[no_mangle]
pub extern "C" fn chaosseal_epoch_crypto(payload: *const u8, payload_len: usize) -> CEpochCryptoResult {
    let payload_slice = unsafe { std::slice::from_raw_parts(payload, payload_len) };
    
    // In a real deployed system, this seed is generated once per epoch asynchronously by chaosseal_epoch_keygen.
    // Here we simulate the per-packet encryption step with a cached derived seed.
    let seed = b"cached_derived_seed_from_keygen_";
    let salt = b"simulated_salt";
    let info = b"simulated_info";
    
    let cipher = crate::crypto::AesGcmCipher::derive_key(seed, salt, info);
    let nonce = crate::crypto::AesGcmCipher::random_nonce();
    
    let ciphertext = cipher.encrypt(payload_slice, &nonce);
    
    CEpochCryptoResult {
        crypto_overhead: ciphertext.len() - payload_len,
    }
}
