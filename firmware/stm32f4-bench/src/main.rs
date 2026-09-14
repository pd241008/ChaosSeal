//! ChaosSeal Cortex-M4F hardware benchmark.
//!
//! Runs the protocol's per-epoch and per-packet hot paths on a Cortex-M4
//! (STM32F405/F407 class, netduinoplus2 under QEMU), using the Q32.32
//! kinematics and crypto sources vendored verbatim from core_v2 @ 29be3b5
//! (merge of PR #20) — see src/vendor/ for the exact deltas (import paths
//! and test modules only; no arithmetic or crypto changes).
//!
//! Timing: SysTick downcounter. On real hardware (168 MHz CLKSOURCE=CPU)
//! it measures true CPU cycles. Under QEMU with `-icount shift=0` the
//! virtual clock advances exactly 1 ns per guest instruction, so SysTick
//! measures deterministic INSTRUCTION counts (the QEMU SoC does not emulate
//! the DWT/ITM; a probe confirmed CYCCNT is frozen there). Run-to-run
//! deltas are exactly reproducible under icount. Reported numbers state
//! which clock they came from: cycles (HW) vs instructions (QEMU-icount,
//! cycles ≈ insns only under an IPC=1 assumption for in-order Cortex-M4).
//!
//! Output: one `[bench]` line per measurement; `[ok]`/`[fail]` lines for
//! the correctness gates (RFC 4231 HMAC KAT, AES-GCM roundtrip) that must
//! pass before any timing is trusted.

#![no_std]
#![no_main]

extern crate alloc;

mod vendor;

use alloc::vec::Vec;
use core::ptr::addr_of_mut;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use linked_list_allocator::LockedHeap;
use panic_semihosting as _;

use vendor::crypto::{aes_gcm::AesGcmCipher, hmac_sha256, hmac_commitment, verify_hmac, derive_packet_key};
use vendor::fixed::Q32_32;
use vendor::kinematics::{MultiPendulum, Rk4Integrator};
use vendor::lyapunov::LyapunovEstimator;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

const HEAP_SIZE: usize = 48 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

const CLOCK_HZ: u32 = 168_000_000; // STM32F407 sysclk; used for us->cycles on HW

/// SysTick counts down from SYST_RVR; mask for elapsed arithmetic.
const SYST_MOD: u32 = 0x00FF_FFFF;

/// Current SysTick downcount. HW: CPU cycles. QEMU(-icount shift=0): insns.
fn sysnow() -> u32 {
    // SAFETY: SYST register block is a fixed MMIO address; read-only here.
    unsafe { (*cortex_m::peripheral::SYST::PTR).cvr.read() }
}

/// Elapsed ticks between two downcounter snapshots (24-bit wrap-safe for
/// gaps < 16.7M ticks). Long measurements MUST be accumulated per-unit.
fn elapsed(start: u32, end: u32) -> u32 {
    start.wrapping_sub(end) & SYST_MOD
}

struct Timer {
    start: u32,
}

impl Timer {
    fn begin() -> Self {
        Self { start: sysnow() }
    }
    fn ticks(&self) -> u32 {
        elapsed(self.start, sysnow())
    }
}

fn us(ticks: u32) -> u64 {
    (ticks as u64 * 1_000_000) / CLOCK_HZ as u64
}

/// Print a failure line and terminate the QEMU/semihosting session.
fn bail(msg: &str) -> ! {
    hprintln!("[fail] {}", msg);
    debug::exit(debug::EXIT_FAILURE);
    loop {
        core::hint::spin_loop();
    }
}

#[entry]
fn main() -> ! {
    // Allocator init (must precede any heap use).
    let heap_ptr = addr_of_mut!(HEAP).cast::<u8>();
    unsafe {
        ALLOCATOR
            .lock()
            .init(heap_ptr.cast(), HEAP_SIZE);
    }

    // SysTick: free-running downcounter, CLKSOURCE=CPU, no interrupt.
    // Under QEMU -icount shift=0 this is the deterministic insn counter.
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let syst = &mut cp.SYST;
    syst.set_reload(SYST_MOD);
    syst.clear_current();
    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.enable_counter();

    // Tick sanity probe: SysTick must be counting (HW: cycles, QEMU: insns).
    // QEMU reloads the downcounter lazily on the first tick after a clear,
    // so spin a few iterations to let the reload land BEFORE snapshotting.
    for _ in 0..1_000u32 {
        core::hint::spin_loop();
    }
    syst.clear_current();
    for _ in 0..10u32 {
        core::hint::spin_loop();
    }
    let csr = syst.csr.read();
    let t0v = sysnow();
    let mut sink = 0u32;
    for i in 0..100_000u32 {
        sink = core::hint::black_box(sink.wrapping_add(core::hint::black_box(i).wrapping_mul(2654435761)));
    }
    core::hint::black_box(&sink);
    let t1v = sysnow();
    let d = elapsed(t0v, t1v);
    hprintln!(
        "[probe] SYST_CSR=0x{:08x} CVR {} -> {} (delta={}) sink={}",
        csr, t0v, t1v, d, sink
    );
    if d == 0 {
        bail("SysTick is not counting; timing would be meaningless");
    }

    hprintln!("[chaosseal-bench] Cortex-M4F Q32.32 protocol benchmark");
    hprintln!("[chaosseal-bench] core_v2 @ 29be3b5, QEMU netduinoplus2, SysTick timing (-icount shift=0 => insns)");

    // ---------------------------------------------------------------
    // Gate 1: HMAC-SHA256 KAT (RFC 4231 case 1)
    // ---------------------------------------------------------------
    let kat_key = [0x0Bu8; 20];
    let kat_msg = b"Hi There";
    // Same constant as core_v2/src/crypto/hmac_sha256.rs (RFC 4231 case 1).
    let kat_expected: [u8; 32] = [
        0xB0, 0x34, 0x4C, 0x61, 0xD8, 0xDB, 0x38, 0x53, 0x5C, 0xA8, 0xAF, 0xCE, 0xAF, 0x0B,
        0xF1, 0x2B, 0x88, 0x1D, 0xC2, 0x00, 0xC9, 0x83, 0x3D, 0xA7, 0x26, 0xE9, 0x37, 0x6C,
        0x2E, 0x32, 0xCF, 0xF7,
    ];
    let got = hmac_sha256::compute(&kat_key, kat_msg);
    if got != kat_expected {
        bail("HMAC-SHA256 RFC 4231 KAT mismatch");
    }
    hprintln!("[ok] HMAC-SHA256 RFC 4231 case 1");

    // ---------------------------------------------------------------
    // Gate 2: AES-256-GCM roundtrip (1024 B payload, deterministic nonce)
    // ---------------------------------------------------------------
    let key: [u8; 32] = core::array::from_fn(|i| i as u8);
    let cipher = AesGcmCipher::new(key);
    let nonce = [0u8; 12];
    let payload: Vec<u8> = alloc::vec![0xAB; 1024];
    let ct = cipher.encrypt(&payload, &nonce);
    if ct.len() != payload.len() + 16 {
        bail("AES-GCM ciphertext length wrong");
    }
    let rt = cipher.decrypt(&ct, &nonce);
    if rt != payload {
        bail("AES-GCM roundtrip mismatch");
    }
    hprintln!("[ok] AES-256-GCM 1024B roundtrip + tag length");

    // ---------------------------------------------------------------
    // Bench 1: epoch stepping (RK4, Q32.32, 3-pendulum wrapped coupling)
    // Canonical params: m=1.0, L=1.0, b=0.02, c=1.0 (wrapped), dt=0.01 s
    // An epoch is 1200 s = 120000 steps; we time 100 steps and extrapolate.
    // ---------------------------------------------------------------
    let pend = MultiPendulum::new(
        3,
        Q32_32::from_f64(1.0),
        Q32_32::from_f64(1.0),
        Q32_32::from_f64(0.02),
        Q32_32::from_f64(1.0),
    );
    let rk4 = Rk4Integrator::new(Q32_32::from_f64(0.01));
    let mut state: Vec<Q32_32> =
        alloc::vec![Q32_32::from_f64(0.1), Q32_32::from_f64(0.5), Q32_32::from_f64(1.0),
                    Q32_32::from_f64(0.3), Q32_32::from_f64(-0.2), Q32_32::from_f64(0.7)];
    let system = |t: Q32_32, s: &[Q32_32]| pend.derivatives(t, s);

    const EPOCH_STEPS: usize = 120_000; // 1200 s / 0.01 s
    const TIMED_STEPS: usize = 100;

    let t0 = Q32_32::ZERO;
    let _ = system(t0, &state); // warm icache/branch predictors
    let mut total: u64 = 0;
    for k in 0..TIMED_STEPS {
        let tm = Timer::begin();
        pend.apply_reinjection(&mut state);
        let (_nt, ns) = rk4.step(&system, t0 + Q32_32::from_f64(k as f64) * rk4.dt, &state);
        state = ns;
        total += tm.ticks() as u64;
    }
    let epoch_step_ticks = (total / TIMED_STEPS as u64) as u32;
    let epoch_cycles = (epoch_step_ticks as u64) * (EPOCH_STEPS as u64);
    hprintln!(
        "[bench] epoch_rk4_step      {:>8} tck/step  {:>6} us/step@HW   (x120000 = {} M-tick/epoch)",
        epoch_step_ticks, us(epoch_step_ticks), epoch_cycles / 1_000_000
    );

    // ---------------------------------------------------------------
    // Bench 2: Lyapunov monitoring (Benettin spectrum, tangent_dim=3)
    // Short horizon (2000 steps) for timing + rough lambda_1 sanity vs
    // design-stage 0.405 nats/s. NOT a final entropy measurement.
    // ---------------------------------------------------------------
    let est = LyapunovEstimator {
        dt: Q32_32::from_f64(0.01),
        steps: 2000,
        reorthonormalize_interval: 10,
        tangent_dim: 3,
    };
    let jacobian = |s: &[Q32_32]| pend.jacobian(s);
    let reinject = |s: &mut [Q32_32]| pend.apply_reinjection(s);
    let ic: Vec<Q32_32> = alloc::vec![Q32_32::from_f64(0.1), Q32_32::from_f64(0.5),
                                     Q32_32::from_f64(1.0), Q32_32::from_f64(0.3),
                                     Q32_32::from_f64(-0.2), Q32_32::from_f64(0.7)];
    // Full run, untimed: lambda_1 sanity check vs design-stage 0.405 nats/s.
    let spectrum = est.estimate_spectrum(&system, &jacobian, &reinject, Q32_32::ZERO, &ic);
    // Timed, single 100-step chunk (< 16.7M SysTick range). The IC is offset
    // from the canonical one so LLVM cannot CSE this run against the untimed
    // full run above (both are side-effect-free); timing-only, not a lambda
    // measurement.
    let mut ic_t = ic.clone();
    ic_t[0] = ic_t[0] + Q32_32::from_f64(0.013);
    const TIMED_LYAP_STEPS: usize = 100;
    let est_t = LyapunovEstimator {
        dt: Q32_32::from_f64(0.01),
        steps: TIMED_LYAP_STEPS,
        reorthonormalize_interval: 10,
        tangent_dim: 3,
    };
    let tm = Timer::begin();
    let _ = est_t.estimate_spectrum(&system, &jacobian, &reinject, Q32_32::ZERO, &ic_t);
    let lyap_ticks = tm.ticks() / TIMED_LYAP_STEPS as u32;
    hprintln!(
        "[bench] lyapunov_benettin   {:>8} tck/step  {:>6} us/step@HW   (lambda_1 ~ {:.3} nats/s @2000 steps, sanity vs 0.405)",
        lyap_ticks, us(lyap_ticks), spectrum[0].to_f64()
    );

    // ---------------------------------------------------------------
    // Bench 3: per-packet crypto path (chaosseal == counter derivation)
    // seed -> HKDF-SHA256 packet key -> AES-256-GCM(1024B) -> HMAC commit
    // ---------------------------------------------------------------
    let session_seed: [u8; 32] = core::array::from_fn(|i| (i as u8).wrapping_mul(7));
    let salt = b"chaosseal-session-salt";
    let info = b"packet-key-v1";
    const PACKET_ITERS: usize = 200;

    // HKDF derive (per-op accumulation; each op << 24-bit range)
    let mut hkdf_total: u64 = 0;
    for c in 0..PACKET_ITERS as u32 {
        let tm = Timer::begin();
        let _k = derive_packet_key(&session_seed, salt, info, c);
        hkdf_total += tm.ticks() as u64;
    }
    let hkdf_ticks = (hkdf_total / PACKET_ITERS as u64) as u32;

    // AES-GCM encrypt 1024 B (key prep inside, matching core AesGcmCipher API)
    let mut last_ct: Vec<u8> = Vec::new();
    let mut gcm_total: u64 = 0;
    for _ in 0..PACKET_ITERS {
        let tm = Timer::begin();
        last_ct = cipher.encrypt(&payload, &nonce);
        gcm_total += tm.ticks() as u64;
    }
    let gcm_ticks = (gcm_total / PACKET_ITERS as u64) as u32;

    // HMAC commitment over ciphertext
    let pkt_key = derive_packet_key(&session_seed, salt, info, 0);
    let mut hmac_total: u64 = 0;
    for _ in 0..PACKET_ITERS {
        let tm = Timer::begin();
        let _tag = hmac_commitment(&pkt_key, &last_ct);
        hmac_total += tm.ticks() as u64;
    }
    let hmac_ticks = (hmac_total / PACKET_ITERS as u64) as u32;

    let packet_ticks = hkdf_ticks as u64 + gcm_ticks as u64 + hmac_ticks as u64;
    hprintln!(
        "[bench] hkdf_packet_key     {:>8} tck/op    {:>6} us/op@HW",
        hkdf_ticks, us(hkdf_ticks)
    );
    hprintln!(
        "[bench] aes256gcm_enc_1024B {:>8} tck/op    {:>6} us/op@HW",
        gcm_ticks, us(gcm_ticks)
    );
    hprintln!(
        "[bench] hmac_commit_1024B   {:>8} tck/op    {:>6} us/op@HW",
        hmac_ticks, us(hmac_ticks)
    );
    hprintln!(
        "[bench] packet_total        {:>8} tck/pkt   {:>6} us/pkt@HW   (HKDF+GCM-enc+HMAC, 1040 B ct)",
        packet_ticks, us(packet_ticks as u32)
    );

    // ---------------------------------------------------------------
    // Gate 3: HMAC verify path roundtrip (protocol-level check)
    // ---------------------------------------------------------------
    let tag = hmac_commitment(&pkt_key, &last_ct);
    if !verify_hmac(&pkt_key, &last_ct, &tag) {
        bail("HMAC commitment verify roundtrip");
    }
    hprintln!("[ok] HMAC commitment verify roundtrip");

    // ---------------------------------------------------------------
    // Protocol feasibility ratios (design-stage; QEMU cycle-derived)
    // ---------------------------------------------------------------
    let pkts_per_epoch = epoch_cycles / packet_ticks.max(1) as u64;
    hprintln!(
        "[ratio] epoch_maintenance_ticks={} per_packet_ticks={} (maintenance == {} packets of budget)",
        epoch_cycles, packet_ticks, pkts_per_epoch
    );

    hprintln!("[done] all gates passed");
    debug::exit(debug::EXIT_SUCCESS);
    // `debug::exit` is not marked diverging in this semihosting version;
    // keep a non-returning tail so `fn main() -> !` typechecks.
    loop {
        core::hint::spin_loop();
    }
}
