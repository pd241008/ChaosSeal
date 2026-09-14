# STM32F4 Hardware Benchmark (QEMU-first, Midas-style flashing)

Cortex-M4F benchmark of the ChaosSeal per-epoch and per-packet hot paths,
running the **core_v2 sources verbatim** (see `src/vendor/` for the exact
deltas — import paths, libm floats, no OS-entropy nonce; zero arithmetic or
crypto changes).

Hardware bring-up (clock, UART, LEDs, flash procedure) follows the **Midas
artifact's** TFLM firmware for the same Discovery-F407VG board.

## Two builds, one source

| Build | Console | Use |
|---|---|---|
| `--features qemu --no-default-features` | semihosting (debugger/QEMU only) | deterministic insn counts under QEMU |
| default (`hw`) | **USART2 @ 115200 8N1 on PA2/PA3** + LEDs PD12..15 | real board, `st-flash` only, no debugger |

Semihosting issues BKPT instructions that hard-fault real hardware without
a debugger attached — hence the UART console for the board, exactly like
Midas's firmware.

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

## Build & flash (STM32F4 Discovery, Midas procedure)

> At-the-board version with wiring, LED codes, expected output, and
> troubleshooting: **`FLASH_CHECKLIST.md`**.

```bash
# WSL: attach ST-LINK first
usbipd bind --hardware-id <VID:PID> && usbipd attach --wsl --hardware-id <VID:PID>

make firmware-hw           # builds default (hw) image -> bench.bin + size report
make firmware-flash        # st-flash write bench.bin 0x08000000
# Console: USART2 @ 115200 8N1 on PA2 (TX)
screen /dev/ttyUSB0 115200   # or: minicom -D /dev/ttyUSB0 -b 115200
```

Linux host: skip usbipd, `st-flash` talks to the ST-LINK directly.

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
DWT cycle counter is enabled and cross-checked against SysTick on hardware
(QEMU lacks DWT emulation for this machine; the QEMU build reports SysTick
only, counting instructions under `-icount shift=0`).

## What the numbers mean

| Clock | Measures | Reproducibility |
|---|---|---|
| QEMU `-icount shift=0` (SysTick) | guest **instructions** | byte-identical across runs (verified) |
| Real HW (SysTick, CLKSOURCE=CPU) | true CPU **cycles** (168 MHz) | ±cache/wait-state jitter |
| Real HW (DWT CYCCNT) | true CPU **cycles** (cross-check) | same |

`cycles ≈ insns` only under an IPC=1 assumption for the in-order Cortex-M4;
expect real-HW cycles **higher** (flash wait states, multi-cycle ops) and
treat QEMU numbers as a *lower bound and relative-cost ranking*.

## Results (QEMU-icount, deterministic)

Correctness gates on target: RFC 4231 HMAC-SHA256 KAT ✅ · AES-256-GCM
1024 B roundtrip ✅ · HMAC commitment verify ✅ · λ₁ sanity ≈ 0.197 nats/s
at 2000 steps (design-stage desktop value 0.405 is a longer-horizon figure).

| Operation | ticks (insns) | µs @168 MHz (IPC=1) |
|---|---:|---:|
| RK4 epoch step (3-pendulum, wrapped c=1.0, dt=10 ms) | 90,704 | 540 |
| Benettin Lyapunov step (tangent_dim=3) | 104,461 | 622 |
| HKDF-SHA256 packet key | 3,743 | 22 |
| AES-256-GCM encrypt 1024 B | 41,757 | 249 |
| HMAC-SHA256 commit over 1040 B ct | 7,896 | 47 |
| **Packet total (derive+encrypt+commit)** | **53,396** | **318** |

Epoch maintenance: 120,000 steps/epoch × 90,704 = **10.88 G ticks/epoch**
(≈ 65 s at 168 MHz IPC=1) ≈ **5.4%** of the 1200 s epoch wall-clock —
equivalent to ~204k packets of budget. The per-packet chain costs ~318 µs
on a 1024 B payload.

Full machine-readable record: `bench_results.json`.
Hardware footprint (default build): text 110.3 KB / BSS 49.2 KB
(1 MB flash / 192 KB SRAM on the F407VG → ~11% / ~26%).

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
