# STM32F4 Hardware Benchmark (QEMU-first)

Cortex-M4F benchmark of the ChaosSeal per-epoch and per-packet hot paths,
running the **core_v2 sources verbatim** (see `src/vendor/` for the exact
deltas — import paths, libm floats, no OS-entropy nonce; zero arithmetic or
crypto changes).

## Why QEMU first

The sandbox has no USB bus, so the physical STM32F4 Discovery cannot be
reached from here. The firmware is **flash-ready**; the workflow is:

1. ✅ **QEMU (done)** — `netduinoplus2` machine (STM32F405 = same M4F core),
   deterministic instruction counts via SysTick under `-icount shift=0`.
2. ⬜ **Hardware (pending)** — flash the same binary to the Discovery board
   and capture DWT CYCCNT numbers (true cycles).

## Build & run

```bash
cd firmware/stm32f4-bench
cargo build --release        # target thumbv7em-none-eabihf

# QEMU (deterministic; instructions counted as ticks)
qemu-system-arm -machine netduinoplus2 -nographic \
  -semihosting-config enable=on,target=native -icount shift=0 \
  -kernel target/thumbv7em-none-eabihf/release/stm32f4-bench

# Real STM32F4 Discovery (ST-Link)
arm-none-eabi-objcopy -O binary \
  target/thumbv7em-none-eabihf/release/stm32f4-bench bench.bin
st-flash write bench.bin 0x08000000
# console: semihosting via openocd, or swap hprintln for SWO/ITM
```

## What the numbers mean

| Clock | Measures | Reproducibility |
|---|---|---|
| QEMU `-icount shift=0` | guest **instructions** (SysTick ticks) | byte-identical across runs (verified) |
| Real HW (168 MHz) | true CPU **cycles** | jitter from flash wait states, caches |

`cycles ≈ insns` only under an IPC=1 assumption for the in-order Cortex-M4;
expect real-HW cycles **higher** (flash wait states, multi-cycle ops) and
treat QEMU numbers as a *lower bound and relative-cost ranking*.

QEMU limitation (probed, documented): the `netduinoplus2` SoC does **not**
emulate the DWT/ITM — `CYCCNT` is frozen — hence the SysTick approach.
SysTick *is* icount-driven and works under QEMU; on real hardware the same
registers measure genuine CPU cycles at CLKSOURCE=CPU.

## Results (QEMU-icount, deterministic)

Correctness gates on target: RFC 4231 HMAC-SHA256 KAT ✅ · AES-256-GCM
1024 B roundtrip ✅ · HMAC commitment verify ✅ · λ₁ sanity ≈ 0.198 nats/s
at 2000 steps (design-stage desktop value 0.405 is a longer-horizon figure).

| Operation | ticks (insns) | µs @168 MHz (IPC=1) |
|---|---:|---:|
| RK4 epoch step (3-pendulum, wrapped c=1.0, dt=10 ms) | 90,703 | 539 |
| Benettin Lyapunov step (tangent_dim=3) | 104,461 | 621 |
| HKDF-SHA256 packet key | 3,771 | 22 |
| AES-256-GCM encrypt 1024 B | 41,757 | 248 |
| HMAC-SHA256 commit over 1040 B ct | 7,896 | 47 |
| **Packet total (derive+encrypt+commit)** | **53,424** | **318** |

Epoch maintenance: 120,000 steps/epoch × 90,703 = **10.88 G ticks/epoch**
(≈ 65 s at 168 MHz IPC=1) ≈ **5.4%** of the 1200 s epoch wall-clock —
equivalent to ~204k packets of budget. The per-packet chain costs ~318 µs
on a 1024 B payload.

Full machine-readable record: `bench_results.json`.

## Integrity notes (why the harness is shaped this way)

- Timing accumulates **per unit** (per step / per op) into `u64`: SysTick is
  24-bit and QEMU ticks at insn granularity — long single windows wrap
  (caught and fixed during development).
- The Benettin timing run uses a **distinct initial condition** from the
  untimed full run so LLVM cannot CSE one into the other (both computations
  are side-effect-free; observed as a 40% under-count before the fix).
- All benchmarks keep their outputs flowing into `black_box`-like sinks
  (verification gates) so nothing is dead-code-eliminated.
