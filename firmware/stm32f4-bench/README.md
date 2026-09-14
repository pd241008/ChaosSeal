# STM32F4 Hardware Benchmark — measured on a Discovery board (QEMU reference included)

Cortex-M4F benchmark of the ChaosSeal per-epoch and per-packet hot paths,
running the **core_v2 sources verbatim** (see `src/vendor/` for the exact
deltas — import paths, libm floats, no OS-entropy nonce; zero arithmetic or
crypto changes).

Hardware bring-up (clock, UART, LEDs, flash procedure) follows the **Midas
artifact's** TFLM firmware for the same Discovery-F407VG board.

**Status: captured on physical hardware (2026-09-14).** All correctness
gates pass on-target, the SysTick and DWT cycle counters agree to 0.0004%,
and two full capture runs are byte-identical. Raw record:
`bench_hw.json` (+ the deterministic QEMU reference in `bench_results.json`).

## Two builds, one source

| Build | Console | Use |
|---|---|---|
| default (`hw`) | **USART2 @ 115200 8N1 on PA2/PA3** + LEDs PD12..15 | real board, `st-flash` only, no debugger |
| `--features qemu --no-default-features` | semihosting (debugger/QEMU only) | deterministic insn counts under QEMU |

Semihosting issues BKPT instructions that hard-fault real hardware without
a debugger attached — hence the UART console for the board, exactly like
Midas's firmware. The `hw` build also mirrors every console line into
reserved SRAM2 with a done flag, so OpenOCD can capture results **without
any USB-serial adapter** (`openocd-capture.cfg`, Midas dump_image pattern).

## Build & run (QEMU — deterministic instruction counts)

```bash
make firmware-bench        # from repo root
# or:
cd firmware/stm32f4-bench
cargo build --release --features qemu --no-default-features
qemu-system-arm -machine netduinoplus2 -nographic \
  -semihosting-config enable=on,target=native -icount shift=0 \
  -kernel target/thumbv7em-none-eabihf/release/stm32f4-bench
```

## Build, flash & capture (STM32F4 Discovery, Midas procedure)

> At-the-board version with wiring, LED codes, expected output, and
> troubleshooting: **`FLASH_CHECKLIST.md`**.

```bash
# WSL: attach ST-LINK first
usbipd bind --hardware-id <VID:PID> && usbipd attach --wsl --hardware-id <VID:PID>

# Serial-free full capture: flash + run + OpenOCD SRAM dump + parse + JSON
make firmware-capture

# Or flash alone and read USART2 on a USB-serial adapter:
make firmware-flash        # st-flash --connect-under-reset write bench.bin 0x08000000
screen /dev/ttyUSB0 115200 # PA2 (TX), 115200 8N1
```

Linux host: skip usbipd, `st-flash` talks to the ST-LINK directly. If the
target refuses to connect (`Can not connect to target`), use
`--connect-under-reset` (already in the Makefile target).

## Hardware

| Pin  | Function           | Notes                                   |
|------|--------------------|------------------------------------------|
| PA2  | USART2 TX          | Console output @ 115200 8N1              |
| PA3  | USART2 RX          | Unused (TX-only console)                 |
| PD12 | LED Green          | ON = running, OFF = done                 |
| PD14 | LED Red            | Steady = bring-up/benchmark failure      |

Clock: HSE 8 MHz → PLL (M=8, N=336, P=2, Q=7) → **SYSCLK 168 MHz**, APB1
42 MHz (USART2 domain), 5 flash wait states + prefetch + caches — identical
constants to Midas `system_init.c`, so `cycles / 168e6` is true seconds.

**On-target clock validation** (built into the firmware): the same
100k-iteration loop is timed twice — SysTick: 1,200,006 ticks, DWT CYCCNT:
1,200,011 cycles (0.0004% apart). The PLL is exactly where the constants
say it is. QEMU lacks DWT emulation for this machine (probed), so its build
reports SysTick only, counting instructions under `-icount shift=0`.

## What the numbers mean

| Clock | Measures | Reproducibility |
|---|---|---|
| Real HW (SysTick, CLKSOURCE=CPU) | true CPU **cycles** (168 MHz) | **byte-identical across runs (verified, ×2)** |
| Real HW (DWT CYCCNT) | true CPU **cycles** (cross-check) | agrees with SysTick to 0.0004% |
| QEMU `-icount shift=0` (SysTick) | guest **instructions** | byte-identical across runs (verified) |

Hardware cycles run **7.3–8.0× above** QEMU instruction counts (flash wait
states, multi-cycle loads/multiplies); the libm-heavy Lyapunov step is the
outlier at 1.14× (FPU + hardware divide). QEMU numbers are a deterministic
reference and relative-cost ranking only — never cycle estimates.

## Results (hardware-measured, 168 MHz)

Correctness gates on target (QEMU **and** hardware): RFC 4231 HMAC-SHA256
KAT ✅ · AES-256-GCM 1024 B roundtrip ✅ · HMAC commitment verify ✅ ·
λ₁ sanity ≈ 0.197 nats/s at 2000 steps — **identical to QEMU and the
desktop core_v2** (Q32.32 bit-exactness across three platforms; the
design-stage 0.405 desktop value is a longer-horizon figure).

| Operation | HW cycles | HW µs | QEMU insns | HW/QEMU |
|---|---:|---:|---:|---:|
| RK4 epoch step (3-pendulum, wrapped c=1.0, dt=10 ms) | 685,378 | 4,080 | 90,704 | 7.6× |
| Benettin Lyapunov step (tangent_dim=3) | 119,155 | 709 | 104,461 | 1.14× |
| HKDF-SHA256 packet key | 28,785 | 171 | 3,743 | 7.7× |
| AES-256-GCM encrypt 1024 B | 334,488 | 1,991 | 41,757 | 8.0× |
| HMAC-SHA256 commit over 1040 B ct | 57,903 | 345 | 7,896 | 7.3× |
| **Packet total (derive+encrypt+commit)** | **421,176** | **2,507** | **53,396** | **7.9×** |

Epoch maintenance: 120,000 steps/epoch × 685,378 cycles =
**82.25 G cycles = 489.6 s ≈ 40.8%** of the 1200 s epoch wall-clock
(equivalent to ~195k packets of budget). The earlier QEMU IPC=1 estimate
(10.88 G ticks, ≈65 s, 5.4%) was off by 7.5× — the measured share is
**~41%**, and this is the number the paper should quote. It is amortized
background work and schedulable, but not negligible. The per-packet chain
costs **2.51 ms** on a 1024 B payload.

Full machine-readable record: `bench_results.json` (raw hardware parse:
`bench_hw.json`).
Hardware footprint (default build): text 112.6 KB / BSS 49.2 KB
(1 MB flash / 112 KB linker RAM → SRAM2 16 KB reserved for the bench log).

## Integrity notes (why the harness is shaped this way)

- Timing accumulates **per unit** (per step / per op) into `u64`: SysTick is
  24-bit and QEMU ticks at insn granularity — long single windows wrap
  (caught and fixed during development).
- The Benettin timing run uses a **distinct initial condition** from the
  untimed full run so LLVM cannot CSE one into the other (both computations
  are side-effect-free; observed as a 40% under-count before the fix).
- All benchmarks keep their outputs flowing into real sinks (verification
  gates, `black_box`ed accumulators) so nothing is dead-code-eliminated.
- HW console is a fixed 128-byte line buffer with integer-only formatting —
  no `fmt` machinery in the timing path.
- SRAM log vs DCache: the debugger's AHB-AP reads do **not** snoop the
  write-back DCache, so every published byte is explicitly DCache-cleaned
  (`DCCMVAC`) before the done flag — otherwise `dump_image` would read
  stale zeros.
