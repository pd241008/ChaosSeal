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
    let lambda1 = estimator.estimate(&|t, s| pendulum.derivatives(t, s), Q32_32::ZERO, &state);

    let cmsg = CString::new("ok").unwrap();
    CChaosSealResult {
        code: 0,
        message: cmsg.into_raw(),
        lambda1: lambda1.to_f64(),
        ciphertext_size: 0,
        dt_bound: (Q32_32::from_f64(0.01) / (lambda1 + Q32_32::from_f64(0.001))).to_f64(),
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
