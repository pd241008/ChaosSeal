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
    // Parameter sweep over non-unit inertia: the m=1.0,L=1.0 default makes
    // inertia=1, so a `-b/I` vs `-b` damping slip (and a missing `/I` on the
    // coupling theta_{i-1} entry) is a no-op and invisible. These configs
    // exercise the damping/coupling entries at inertia in {.25, .5, 1.125, 6}.
    let configs = [
        (1.0f64, 1.0f64, 0.1f64, 0.5f64),
        (1.0f64, 0.5f64, 0.1f64, 0.5f64),
        (0.5f64, 1.0f64, 0.1f64, 0.5f64),
        (2.0f64, 0.75f64, 0.4f64, 0.3f64),
        (1.5f64, 2.0f64, 0.2f64, 0.7f64),
    ];
    let state: Vec<Q32_32> = [0.5, -0.7, 0.3, 0.9, -0.4, 0.2]
        .iter().map(|&v| Q32_32::from_f64(v)).collect();
    for &(m, l, b, c) in configs.iter() {
        let pendulum = MultiPendulum::new(3, Q32_32::from_f64(m), Q32_32::from_f64(l), Q32_32::from_f64(b), Q32_32::from_f64(c));
        let inertia = m * l * l;
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
                    "m={m} L={l} (inertia {inertia:.3}): jacobian[{r}][{c}] analytic {a:.6} vs finite-diff {d:.6} (rel {:.4})",
                    err / scale);
            }
        }
        eprintln!("jacobian FD check (m={m} L={l} inertia {inertia:.3}): worst rel {worst:.4} at {worst_rc:?}");
    }
}

#[test]
fn test_tangent_product_identity() {
    // J(x)v must equal the directional derivative (f(x+eps v) - f(x))/eps:
    // the core identity the Benettin tangent update relies on. Run at several
    // inertias so the damping row (-b, not -b/I) and coupling entries are hit.
    let configs = [
        (1.0f64, 1.0f64, 0.1f64, 0.5f64),
        (1.0f64, 0.5f64, 0.1f64, 0.5f64),
        (0.5f64, 1.0f64, 0.1f64, 0.5f64),
        (1.5f64, 2.0f64, 0.2f64, 0.7f64),
    ];
    let state: Vec<Q32_32> = [0.5, -0.7, 0.3, 0.9, -0.4, 0.2]
        .iter().map(|&v| Q32_32::from_f64(v)).collect();
    let v: Vec<f64> = [0.1, -0.5, 0.8, 0.3, -0.2, 0.6].to_vec();
    let eps = 1e-3;
    for &(m, l, b, c) in configs.iter() {
        let pendulum = MultiPendulum::new(3, Q32_32::from_f64(m), Q32_32::from_f64(l), Q32_32::from_f64(b), Q32_32::from_f64(c));
        let jac = pendulum.jacobian(&state);
        let mut jv = vec![0f64; state.len()];
        for r in 0..state.len() {
            for cc in 0..state.len() {
                jv[r] += jac[r][cc].to_f64() * v[cc];
            }
        }
        let mut sp = state.to_vec();
        let mut sm = state.to_vec();
        for cc in 0..state.len() {
            sp[cc] = sp[cc] + Q32_32::from_f64(eps * v[cc]);
            sm[cc] = sm[cc] - Q32_32::from_f64(eps * v[cc]);
        }
        let fp = pendulum.derivatives(Q32_32::ZERO, &sp);
        let fm = pendulum.derivatives(Q32_32::ZERO, &sm);
        for r in 0..state.len() {
            let fd_directional = (fp[r].to_f64() - fm[r].to_f64()) / (2.0 * eps);
            assert!((jv[r] - fd_directional).abs() <= jv[r].abs().max(1e-6) * 0.02 + 1e-4,
                "m={m} L={l}: J*v[{r}]: {:.6} vs directional FD {:.6}", jv[r], fd_directional);
        }
    }
    eprintln!("tangent product identity Jv = (f(x+ev)-f(x-ev))/2e verified at multiple inertias");
}

#[test]
fn test_wrapped_coupling_jacobian_off_cut() {
    // The wrapped elastic coupling wrap(d) = atan2(sin d, cos d) is locally
    // linear with unit slope, so the analytic Jacobian keeps the pre-wrapped
    // coupling entries (+/- c*0.1/d) for every state strictly away from the
    // branch cut at odd multiples of pi (and further than the FD half-step
    // delta). Sweep angle pairs that land on different branches: small
    // differences, over-the-top (diff ~ pi-0.05), just past-the-top (diff ~
    // pi+0.05, wraps to -(pi-0.05)), and multi-turn spins.
    let pendulum = MultiPendulum::new(3, Q32_32::from_f64(1.0), Q32_32::from_f64(1.0), Q32_32::from_f64(0.1), Q32_32::from_f64(1.0));
    let states: Vec<Vec<f64>> = vec![
        vec![0.5, -0.7, 0.3, 0.9, -0.4, 0.2],         // diffs -1.2, 1.0
        vec![0.0, 3.09159, 0.0, 0.0, 0.0, 0.0],      // diff pi-0.05 (below cut)
        vec![0.0, 3.19159, 2.0, 0.0, 0.0, 0.0],      // diff pi+0.05 (above cut)
        vec![-6.5, 0.0, 6.4, 0.0, 0.0, 0.0],         // diffs 6.5, -6.4 (multi-turn)
    ];
    for state_f in states {
        let state: Vec<Q32_32> = state_f.iter().map(|&v| Q32_32::from_f64(v)).collect();
        let analytic = pendulum.jacobian(&state);
        let fd = finite_difference_jacobian(&pendulum, &state, 1e-3);
        let mut worst = 0f64;
        let mut worst_rc = (0usize, 0usize);
        for r in 0..state.len() {
            for c in 0..state.len() {
                let a = analytic[r][c].to_f64();
                let d = fd[r][c];
                let scale = a.abs().max(1e-6);
                let err = (a - d).abs();
                if err / scale > worst {
                    worst = err / scale;
                    worst_rc = (r, c);
                }
                assert!(err <= scale * 0.02 + 1e-4,
                    "state {state_f:?}: jacobian[{r}][{c}] analytic {a:.6} vs FD {d:.6}",
                    );
            }
        }
        eprintln!("wrapped-coupling Jacobian FD (state {state_f:?}): worst rel {worst:.4} at {worst_rc:?}");
    }
}

#[test]
fn test_wrapped_coupling_branch_cut_behavior() {
    // At the exact branch cut (theta_i - theta_j = odd multiple of pi) the
    // wrapped coupling is DISCONTINUOUS: a spring turns over, torque jumps by
    // ~2*pi*C*0.1/d (finite). The vector field stays finite, so the ODE is
    // well-behaved; Benettin only ever needs the slope-1 Jacobian (valid
    // almost everywhere, and RK4 steps virtually never land exactly on the
    // measure-zero cut). This test pins that contract and documents that an
    // FD probe straddling the cut must NOT reproduce slope 1 -- that mismatch
    // is the expected discontinuity, not a regression.
    let pendulum = MultiPendulum::new(3, Q32_32::from_f64(1.0), Q32_32::from_f64(1.0), Q32_32::from_f64(0.1), Q32_32::from_f64(1.0));
    let state: Vec<Q32_32> = [0.0, std::f64::consts::PI, 0.0, 0.0, 0.0, 0.0]
        .iter().map(|&v| Q32_32::from_f64(v)).collect();

    // 1) ODE finite (and not absurdly large) at the cut.
    let d = pendulum.derivatives(Q32_32::ZERO, &state);
    for v in &d {
        assert!(v.to_f64().is_finite(), "deriv value non-finite at the branch cut");
    }
    for i in 3..6 {
        assert!(d[i].to_f64().abs() < 20.0, "omega deriv {i} out of band at cut: {}", d[i].to_f64());
    }

    // 2) The pure coupling entry (omega_1 w.r.t. theta_0) keeps slope 1: -c*0.1/d = -0.1.
    let analytic = pendulum.jacobian(&state);
    assert!((analytic[4][0].to_f64() + 0.1).abs() < 1e-3,
        "analytic coupling entry at the cut should stay -0.1, got {}", analytic[4][0].to_f64());

    // 3) FD straddling the cut diverges from slope 1 (documented discontinuity).
    let fd = finite_difference_jacobian(&pendulum, &state, 1e-3);
    assert!((fd[4][0] + 0.1).abs() > 1.0,
        "FD across the branch cut must NOT match slope 1 (jump ~2*pi*0.1), got {:.3}", fd[4][0]);
    eprintln!("branch cut: ODE finite, slope-1 analytic = -0.1, straddling FD = {:.1} (discontinuity confirmed)", fd[4][0]);
}

#[test]
fn test_wrapped_coupling_bounded_under_spin() {
    // Boundedness at the coupling level: the raw linear term drives the
    // elastic torque unboundedly as the relative angle accumulates multiple
    // turns; the wrapped term confines |torque_c| <= C*pi/d*0.1 regardless of
    // the relative angle, so spinning states cannot pump the coupling energy.
    let pendulum = MultiPendulum::new(3, Q32_32::from_f64(1.0), Q32_32::from_f64(1.0), Q32_32::from_f64(0.1), Q32_32::from_f64(1.0));
    let bound = 1.0 * std::f64::consts::PI / 1.0 * 0.1 + 1e-3;
    for spins in [0u32, 1, 8, 64, 512, 2048] {
        let offset = spins as f64 * 2.0 * std::f64::consts::PI + 0.313;
        let state: Vec<Q32_32> = [0.0, offset, 0.0, 0.0, 0.0, 0.0]
            .iter().map(|&v| Q32_32::from_f64(v)).collect();
        let d = pendulum.derivatives(Q32_32::ZERO, &state);
        let tau_c = d[4].to_f64(); // omega_1 row = gravity + coupling; gravity |.|<=9.8
        assert!(tau_c.is_finite(), "non-finite torque at {spins} turns");
        assert!(tau_c.abs() < 9.8 + 1.0 * bound + 1e-3,
            "coupling torque escaped the wrap bound at {spins} turns: {tau_c:.4}");
        eprintln!("wrapped coupling at {spins} turns: |torque| <= {:.5} (bound materialized)", tau_c.abs());
    }
}

#[test]
fn test_lyapunov_spectrum_lambda1_wrapped_coupling() {
    // Bounded (wrapped-atan2) coupling at the new default c=1.0. lambda_1 at
    // the deterministic IC [0.1,0.2,0.3] over T=100 s (dt=0.01, reinjection)
    // is cross-validated by scripts/validate_benettin.py, which replicates the
    // Rust integrator (ODE, wrapped coupling, Jacobian, RK4, reorthonormalize
    // schedule) in float64: ref 0.23974 vs Rust Q32.32 0.22885 (~4.5%, the
    // documented fixed-point trig bias). The old "transient window" framing of
    // the linear coupling (energy escape in ~17-130 s, see
    // docs/design_note_metastability.md) no longer applies: the wrapped term
    // is bounded by construction, so long-horizon spectra are integrable.
    let pendulum = MultiPendulum::new(3, Q32_32::from_f64(1.0), Q32_32::from_f64(1.0), Q32_32::from_f64(0.1), Q32_32::from_f64(1.0));
    let mut state = vec![Q32_32::ZERO; pendulum.dimension()];
    for i in 0..3 {
        state[i] = Q32_32::from_f64(0.1 * (i as f64 + 1.0));
    }
    let estimator = LyapunovEstimator { steps: 10000, ..Default::default() };
    let spec = estimator.estimate_spectrum(
        &|t, s| pendulum.derivatives(t, s),
        &|s| pendulum.jacobian(s),
        &|s| pendulum.apply_reinjection(s),
        Q32_32::ZERO, &state);
    let s: Vec<f64> = spec.iter().map(|x| x.to_f64()).collect();
    assert!(s.len() == 3, "expected 3-D spectrum, got {:?}", s);
    assert!(s[0] >= 0.0, "lambda1 must be non-negative, got {s:?}");
    assert!((s[0] - 0.22885).abs() < 0.03,
        "lambda1 {:.4} must be within 0.03 of the validated 0.2289 (float64 ref 0.2397)", s[0]);
    let ks: f64 = s.iter().filter(|&&x| x > 0.0).sum();
    assert!(ks >= s[0], "KS entropy must include lambda1 when it is positive");
}

#[test]
fn probe_spectrum_horizon_dependence() {
    // Temporary diagnostic: how does the top-3 spectrum at the deterministic IC
    // evolve with the integration horizon under the bounded (wrapped) coupling
    // at the new default c=1.0, and does reinjection change it?
    let pendulum = MultiPendulum::new(3, Q32_32::from_f64(1.0), Q32_32::from_f64(1.0), Q32_32::from_f64(0.1), Q32_32::from_f64(1.0));
    let with_inject = |s: &mut [Q32_32]| {
        let n = pendulum.dimension() / 2;
        let mut sum = Q32_32::from_f64(0.0);
        for i in 0..n { sum = sum + s[n + i].abs(); }
        if sum < Q32_32::from_f64(0.5) { s[n] = s[n] + Q32_32::from_f64(3.0); }
    };
    fn run_rej(pendulum: &MultiPendulum, rej: &dyn Fn(&mut [Q32_32]), steps: usize) {
        let mut state = vec![Q32_32::ZERO; pendulum.dimension()];
        for i in 0..3 { state[i] = Q32_32::from_f64(0.1 * (i as f64 + 1.0)); }
        let estimator = LyapunovEstimator { steps, ..Default::default() };
        let spec = estimator.estimate_spectrum(
            &|t, s| pendulum.derivatives(t, s),
            &|s| pendulum.jacobian(s),
            rej, Q32_32::ZERO, &state);
        let s: Vec<f64> = spec.iter().map(|x| x.to_f64()).collect();
        let w = estimator.estimate(
            &|t, s| pendulum.derivatives(t, s),
            &|s| pendulum.jacobian(s),
            rej, Q32_32::ZERO, &state);
        let _ = w;
        eprintln!("steps={steps:6} spec=({:.5},{:.5},{:.5}) ks_pos={:.5}",
            s[0], s[1], s[2], s.iter().filter(|&&x| x > 0.0).sum::<f64>());
    }
    for steps in [10000usize, 50000, 100000, 200000, 400000, 800000] {
        run_rej(&pendulum, &with_inject, steps);
    }
}

#[test]
fn probe_fixed_point_trig_accuracy_vs_argument() {
    // Diagnostic: does Q32_32 sin/cos accuracy degrade with |arg|? If the
    // approximant only performs range-reduction well within a small window,
    // long-horizon trajectories (theta wanders beyond +/-pi) may corrupt.
    for (label, arg) in [
        ("0.3", 0.3f64), ("1.0", 1.0), ("3.1", 3.1), ("6.0", 6.0),
        ("32.0", 32.0), ("128.0", 128.0), ("1024.0", 1024.0),
        ("4096.0", 4096.0), ("100000.0", 100000.0),
    ] {
        let q = Q32_32::from_f64(arg);
        let vs = (q.sin().to_f64() - arg.sin()).abs();
        let vc = (q.cos().to_f64() - arg.cos()).abs();
        eprintln!("arg={label:>9}:  |sin err|={vs:.3e}  |cos err|={vc:.3e}");
    }
}

#[test]
fn test_bee_ciphertext_size_scaling() {
    let sizes: Vec<usize> = (1..=16).map(|r| BEEEngine::new(1024, r).ciphertext_size_min()).collect();
    for (i, &size) in sizes.iter().enumerate() {
        assert!(size > 0, "Ciphertext size must be positive for r={}", i + 1);
        assert!(size < 100000, "Ciphertext size must be reasonable for r={}", i + 1);
    }
}
