//! ChaosSeal Cortex-M4F hardware benchmark.
//!
//! Runs the protocol's per-epoch and per-packet hot paths on a Cortex-M4
//! (STM32F405/F407 class), using the Q32.32 kinematics and crypto sources
//! vendored verbatim from core_v2 @ 29be3b5 (merge of PR #20) — see
//! src/vendor/ for the exact deltas (import paths and test modules only;
//! no arithmetic or crypto changes).
//!
//! Console backends (Midas-artifact style, feature-selected):
//! - `hw` (default): bare-metal USART2 @ 115200 8N1 on PA2/PA3, status LEDs
//!   PD12(green)/PD14(red), PLL 168 MHz clock init ported from the Midas
//!   firmware. Works with `st-flash write` alone — no debugger needed.
//! - `qemu`: semihosting console + exit for `make firmware-bench` under
//!   QEMU (`--features qemu`). Semihosting BKPTs would hard-fault real
//!   hardware without a debugger, hence the split.
//!
//! Timing: SysTick downcounter, per-unit accumulation into u64 (24-bit
//! counter). On hardware (168 MHz, CLKSOURCE=CPU) ticks are true CPU
//! cycles and the DWT is enabled as a cross-check; under QEMU
//! `-icount shift=0` ticks are deterministic guest instructions (QEMU
//! does not emulate the DWT for this machine — probed). All reported
//! numbers state which clock produced them.

#![no_std]
#![no_main]

extern crate alloc;

mod vendor;
#[cfg(feature = "hw")]
mod hw;

use alloc::vec::Vec;
use core::ptr::addr_of_mut;
use cortex_m_rt::entry as entry_macro;
use linked_list_allocator::LockedHeap;

#[cfg(all(feature = "qemu", not(feature = "hw")))]
use cortex_m_semihosting::{debug, hprintln};
#[cfg(all(feature = "qemu", not(feature = "hw")))]
use panic_semihosting as _;
// HW panic handling: custom #[panic_handler] in src/hw.rs (red LED + halt).

use vendor::crypto::{
    aes_gcm::AesGcmCipher, derive_packet_key, hmac_commitment, hmac_sha256, verify_hmac,
};
use vendor::fixed::Q32_32;
use vendor::kinematics::{MultiPendulum, Rk4Integrator};
use vendor::lyapunov::LyapunovEstimator;

/// failtest: flip one bit of the expected KAT tag so Gate 1 must fail. This
/// exercises the failure path itself — console `[fail]` line, red LED (HW),
/// non-zero QEMU exit — proving the gates can actually fail.

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

const HEAP_SIZE: usize = 48 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

/// CPU frequency after `hw::clock_init_168mhz` (Midas PLL constants).
/// Tick->µs conversion happens offline in analysis; kept for HW-path helpers.
#[allow(dead_code)]
const CLOCK_HZ: u32 = 168_000_000;

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

// ---------------------------------------------------------------------------
// Console abstraction over the two backends
// ---------------------------------------------------------------------------
#[cfg(all(feature = "qemu", not(feature = "hw")))]
mod console {
    use super::*;
    pub fn init() {}
    #[inline]
    pub fn ok(line: &str) {
        hprintln!("[ok] {}", line);
    }
    #[inline]
    pub fn info(line: &str) {
        hprintln!("{}", line);
    }
    #[inline]
    pub fn bench(label: &str, ticks: u64, tag: &str) {
        hprintln!("[bench] {} {} {}", label, ticks, tag);
    }
    pub fn bail(msg: &str) -> ! {
        hprintln!("[fail] {}", msg);
        debug::exit(debug::EXIT_FAILURE);
        loop {
            core::hint::spin_loop();
        }
    }
    pub fn done() -> ! {
        hprintln!("[done] all gates passed");
        debug::exit(debug::EXIT_SUCCESS);
        loop {
            core::hint::spin_loop();
        }
    }
}

#[cfg(all(feature = "hw", not(feature = "qemu")))]
mod console {
    use super::hw;

    // Small fixed buffer for formatted lines (no formatting machinery on HW).
    const BUFSZ: usize = 128;
    struct Line {
        buf: [u8; BUFSZ],
        len: usize,
    }

    impl Line {
        fn new() -> Self {
            Self {
                buf: [0; BUFSZ],
                len: 0,
            }
        }
        fn push(&mut self, s: &str) {
            for b in s.as_bytes() {
                if self.len < BUFSZ {
                    self.buf[self.len] = *b;
                    self.len += 1;
                }
            }
        }
        fn push_u64(&mut self, mut v: u64) {
            let mut tmp = [0u8; 20];
            let mut i = tmp.len();
            if v == 0 {
                tmp[19] = b'0';
                i = 19;
            }
            while v > 0 {
                i -= 1;
                tmp[i] = b'0' + (v % 10) as u8;
                v /= 10;
            }
            self.push(core::str::from_utf8(&tmp[i..]).unwrap_or("?"));
        }
        fn send(&self) {
            for i in 0..self.len {
                hw::uart_putc(self.buf[i]);
            }
        }
    }

    pub fn init() {
        hw::clock_init_168mhz();
        hw::dwt_enable();
        hw::led_init();
        hw::uart2_init();
        // Boot LED: green ON until done (Midas pattern: PD12 boot/done).
        hw::led_on(0);
    }

    pub fn ok(line: &str) {
        let mut l = Line::new();
        l.push("[ok] ");
        l.push(line);
        l.send();
        l.push("\r\n");
        l.send();
    }

    pub fn info(line: &str) {
        let mut l = Line::new();
        l.push(line);
        l.push("\r\n");
        l.send();
    }

    pub fn bench(label: &str, ticks: u64, tag: &str) {
        let mut l = Line::new();
        l.push("[bench] ");
        l.push(label);
        l.push(" ");
        l.push_u64(ticks);
        l.push(" ");
        l.push(tag);
        l.push("\r\n");
        l.send();
    }

    pub fn bail(msg: &str) -> ! {
        let mut l = Line::new();
        l.push("[fail] ");
        l.push(msg);
        l.push("\r\n");
        l.send();
        hw::led_off(0); // green off
        hw::led_on(2); // red steady (Midas error indicator)
        loop {
            core::hint::spin_loop();
        }
    }

    pub fn done() -> ! {
        let mut l = Line::new();
        l.push("[done] all gates passed\r\n");
        l.send();
        hw::led_off(0); // green off = done (Midas pattern)
        loop {
            core::hint::spin_loop();
        }
    }
}

use console::{bail, bench as bench_line, done, info, ok};

#[entry_macro]
fn main() -> ! {
    // Allocator init (must precede any heap use).
    let heap_ptr = addr_of_mut!(HEAP).cast::<u8>();
    unsafe {
        ALLOCATOR
            .lock()
            .init(heap_ptr.cast(), HEAP_SIZE);
    }

    // Console/clock bring-up (HW: PLL 168 MHz + UART + LEDs; QEMU: no-op).
    console::init();

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
    // so spin briefly before snapshotting.
    for _ in 0..1_000u32 {
        core::hint::spin_loop();
    }
    syst.clear_current();
    for _ in 0..10u32 {
        core::hint::spin_loop();
    }
    let t0v = sysnow();
    let mut sink = 0u32;
    for i in 0..100_000u32 {
        sink = core::hint::black_box(
            sink.wrapping_add(core::hint::black_box(i).wrapping_mul(2654_435_761)),
        );
    }
    core::hint::black_box(&sink);
    let t1v = sysnow();
    let d = elapsed(t0v, t1v);
    #[cfg(all(feature = "hw", not(feature = "qemu")))]
    {
        let dwt0 = hw::dwt_cycles();
        let mut sink2 = 0u32;
        for i in 0..100_000u32 {
            sink2 = core::hint::black_box(
                sink2.wrapping_add(core::hint::black_box(i).wrapping_mul(2654_435_761)),
            );
        }
        core::hint::black_box(&sink2);
        let dwt1 = hw::dwt_cycles();
        info("[probe] dual-clock check: SysTick vs DWT deltas follow");
        bench_line("systick_probe", d as u64, "tck");
        bench_line("dwt_probe", dwt1.wrapping_sub(dwt0) as u64, "cyc");
    }
    if d == 0 {
        bail("SysTick is not counting; timing would be meaningless");
    }

    info("[chaosseal-bench] Cortex-M4F Q32.32 protocol benchmark");
    info("[chaosseal-bench] core_v2 @ 29be3b5; SysTick timing (HW: cycles, QEMU: insns)");

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
    #[cfg(feature = "failtest")]
    let kat_expected: [u8; 32] = {
        let mut e = kat_expected;
        e[0] ^= 0x01;
        e
    };
    if got != kat_expected {
        bail("HMAC-SHA256 RFC 4231 KAT mismatch");
    }
    ok("HMAC-SHA256 RFC 4231 case 1");

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
    ok("AES-256-GCM 1024B roundtrip + tag length");

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
    let mut state: Vec<Q32_32> = alloc::vec![
        Q32_32::from_f64(0.1),
        Q32_32::from_f64(0.5),
        Q32_32::from_f64(1.0),
        Q32_32::from_f64(0.3),
        Q32_32::from_f64(-0.2),
        Q32_32::from_f64(0.7),
    ];
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
    bench_line("epoch_rk4_step", epoch_step_ticks as u64, "tck/step");
    bench_line("epoch_rk4_epoch", epoch_cycles, "tck/epoch");

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
    let ic: Vec<Q32_32> = alloc::vec![
        Q32_32::from_f64(0.1),
        Q32_32::from_f64(0.5),
        Q32_32::from_f64(1.0),
        Q32_32::from_f64(0.3),
        Q32_32::from_f64(-0.2),
        Q32_32::from_f64(0.7),
    ];
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
    bench_line("lyapunov_benettin", lyap_ticks as u64, "tck/step");
    // Integer millinats to stay float-free on the HW console path.
    let lam1_millinats = (spectrum[0].to_f64() * 1000.0) as i64;
    bench_line("lambda1_sanity_millinats", lam1_millinats as u64, "mnats/s @2000steps");

    // ---------------------------------------------------------------
    // Bench 3: per-packet crypto path (chaosseal == counter derivation)
    // seed -> HKDF-SHA256 packet key -> AES-256-GCM(1024B) -> HMAC commit
    // ---------------------------------------------------------------
    let session_seed: [u8; 32] = core::array::from_fn(|i| (i as u8).wrapping_mul(7));
    let salt = b"chaosseal-session-salt";
    let info_str = b"packet-key-v1";
    const PACKET_ITERS: usize = 200;

    // HKDF derive (per-op accumulation; each op << 24-bit range)
    let mut hkdf_total: u64 = 0;
    for c in 0..PACKET_ITERS as u32 {
        let tm = Timer::begin();
        let _k = derive_packet_key(&session_seed, salt, info_str, c);
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
    let pkt_key = derive_packet_key(&session_seed, salt, info_str, 0);
    let mut hmac_total: u64 = 0;
    for _ in 0..PACKET_ITERS {
        let tm = Timer::begin();
        let _tag = hmac_commitment(&pkt_key, &last_ct);
        hmac_total += tm.ticks() as u64;
    }
    let hmac_ticks = (hmac_total / PACKET_ITERS as u64) as u32;

    let packet_ticks = hkdf_ticks as u64 + gcm_ticks as u64 + hmac_ticks as u64;
    bench_line("hkdf_packet_key", hkdf_ticks as u64, "tck/op");
    bench_line("aes256gcm_enc_1024B", gcm_ticks as u64, "tck/op");
    bench_line("hmac_commit_1024B", hmac_ticks as u64, "tck/op");
    bench_line("packet_total", packet_ticks, "tck/pkt");

    // ---------------------------------------------------------------
    // Gate 3: HMAC verify path roundtrip (protocol-level check)
    // ---------------------------------------------------------------
    let tag = hmac_commitment(&pkt_key, &last_ct);
    if !verify_hmac(&pkt_key, &last_ct, &tag) {
        bail("HMAC commitment verify roundtrip");
    }
    ok("HMAC commitment verify roundtrip");

    // ---------------------------------------------------------------
    // Protocol feasibility ratios (design-stage)
    // ---------------------------------------------------------------
    let pkts_per_epoch = epoch_cycles / packet_ticks.max(1);
    bench_line("epoch_maintenance_ticks", epoch_cycles, "tck/epoch");
    bench_line("per_packet_ticks", packet_ticks, "tck/pkt");
    bench_line("epoch_over_packet_budget", pkts_per_epoch, "pkts");

    done();
}
