# Reproduction

This guide covers **full reproduction** of the artifact's experiments from
scratch. For fast verification without re-running experiments, see `VERIFY.md`.

## Prerequisites

```bash
rustc --version   # >= 1.75
go version        # >= 1.22
python3 --version # >= 3.10
pip install -r requirements.txt
```

## Build

```bash
# Rust protocol core (source of truth for all crypto/kinematics)
cd core_v2 && cargo build --release && cd ..

# Go network simulator (CGO links libchaosseal_core)
cd netsim_v2 && CGO_ENABLED=1 go build -o chaoseal-sim . && cd ..

# (optional) Cortex-M4 no_std firmware benchmark; needs the
# thumbv7em-none-eabihf target: rustup target add thumbv7em-none-eabihf
cd firmware/stm32f4-bench && cargo build --release && cd ..
```

Optional: with `qemu-system-arm` installed, `make firmware-bench` runs the
firmware under emulation (deterministic instruction counts; see
`firmware/stm32f4-bench/README.md`).

Or run everything through the pipeline driver, which builds both:

```bash
./run_sweep_v3.sh results_v3_repro
```

## Experiment Reproduction

### Tier 1a — Goodput R-sweep: chaosseal vs counter vs BPSec (Claim C1, C6)

N=1024 satellites, epoch=1200 s, 5 seeds × 10 revoked-set sizes
R ∈ {1,2,4,8,16,32,64,128,256,512} = 50 runs:

```bash
./run_sweep_v3.sh results_v3_repro
```

(Runtimes: ~0.3 s per run; the full driver including all tiers is minutes.)

Direct single runs:

```bash
RUSTCLI="$(pwd)/core_v2/target/release/chaosseal"
./netsim_v2/chaoseal-sim --run-id my-rsweep-seed1-r0008 --seed 1 --bee-r 8 \
  --baselines chaosseal,counter,bpsec --results-dir results_fresh \
  --rust-cli "$RUSTCLI"
```

### Tier 2a — Packet-loss sweep (Claim C8 context)

```bash
for loss in 0.0 0.01 0.03 0.05 0.10; do
  lbl=$(python3 -c "print(f'{${loss}:.2f}'.replace('.','p'))")
  ./netsim_v2/chaoseal-sim --run-id "loss-sweep-${lbl}" --seed 1 --bee-r 8 \
    --baselines chaosseal,counter,bpsec --loss-rate $loss \
    --results-dir results_fresh --rust-cli "$(pwd)/core_v2/target/release/chaosseal"
done
```

### Tier 2b — Packet-size sweep (Claim C9)

```bash
for size in 128 256 512 1024 2048 4096; do
  ./netsim_v2/chaoseal-sim --run-id "size-sweep-${size}" --seed 1 --bee-r 8 \
    --baselines chaosseal,counter,bpsec --payload-bytes $size \
    --results-dir results_fresh --rust-cli "$(pwd)/core_v2/target/release/chaosseal"
done
```

### Tier 2c — Commitment interval N (Claim C5 input)

```bash
for n in 1 4 16 64 256 1024; do
  ./netsim_v2/chaoseal-sim --run-id "commit-sweep-n${n}" --seed 1 --bee-r 8 \
    --baselines chaosseal --commit-interval $n \
    --results-dir results_fresh --rust-cli "$(pwd)/core_v2/target/release/chaosseal"
done
```

### Single-bit corruption detection (Claim C2)

```bash
./netsim_v2/chaoseal-sim --run-id corruption-test --corruption-test \
  --results-dir results_fresh --rust-cli "$(pwd)/core_v2/target/release/chaosseal"
```

256 bit positions × 64 packets/epoch, give-up threshold 256 epochs.

### Dynamic membership / join protocol (Claim C7)

```bash
./netsim_v2/chaoseal-sim --run-id membership-join --membership-test \
  --membership-joins 8 --results-dir results_fresh \
  --rust-cli "$(pwd)/core_v2/target/release/chaosseal"

# Cost-model sweep via the Rust CLI:
./core_v2/target/release/cli_v2 join-protocol --n 1024 --r 8 --joins 20 \
  --rebuild-interval-epochs 6.0 --epoch-duration-s 1200
```

### Lyapunov spectrum + finite-horizon convergence (Claim C4)

```bash
# Full spectrum (λ₁, λ₂, λ₃) + Kolmogorov-Sinai entropy, one horizon:
./core_v2/target/release/cli_v2 lyapunov-spectrum --pendulums 3 --mass 1.0 \
  --length 1.0 --damping 0.1 --coupling 1.0 --steps 400000 --samples 100

# All horizons + convergence table (dt=0.01 ⇒ --steps 200000 = T=2000 s):
python3 scripts/verify_spectrum_convergence.py
```

Expected runtime: ~1.5 s at T=200 s (20 samples); the archived T=4000 s /
100-sample horizon takes minutes on a single core.

### Pendulum robustness + λ_min distribution (Claim C3, C10)

```bash
python3 scripts/regen_lambda_min_series.py   # 10 trials × 1000 draws
python3 scripts/regen_robustness.py          # parameter sweep, SWEEP_WORKERS parallel
python3 scripts/sample_lyapunov.py           # λ sampling via the Rust CLI
```

### Verification gates (C3 correctness)

```bash
python3 scripts/validate_benettin.py     # float64 replicator vs Rust Q32.32 → ALL MATCH
python3 scripts/verify_metastability.py  # linear-coupling bounds proof (historical)
```

### Commitment interval × LEO loss profiles (Claim C5)

```bash
python3 analysis/commit_loss_sweep.py    # writes results_v3/v3_commit_loss_sweep_stats.csv + figures
```

### Bootstrap CI on the crossover point (Claim C6)

```bash
python3 analysis/bootstrap_crossover.py  # 10,000 resamples over the 5-seed R-sweep
```

### Burst-then-recover event model (paper Figure 7)

```bash
python3 analysis/burst_model.py          # writes figures/burst_goodput_time.pdf
```

### Aggregation and figures (all claims)

```bash
python3 analysis/v3_analysis.py          # stats CSVs + results_v3/figures/*.pdf
python3 analysis/v4_generalization.py    # λ_min distribution, crossover surface, keystream entropy
```

## Legacy-generation sweeps (provenance only)

`run_sweep.sh` and `run_sweep_v2.sh` drive the superseded `core/`+`netsim/`
generation. They are retained for provenance and are **not** part of any
canonical claim; see `PROVENANCE.md`.

### Cortex-M4 firmware benchmark (hardware-cost baseline)

The per-epoch and per-packet hot paths run unmodified on a Cortex-M4F
(sources vendored verbatim from core_v2; see the firmware README for the
exact, documented deltas):

```bash
make firmware-bench
```

**Expected runtime**: seconds to build; QEMU run is fully deterministic.

**Expected output** (QEMU 11.x, `-machine netduinoplus2 -icount shift=0`):
three `[ok]` gate lines (RFC 4231 HMAC KAT, AES-GCM roundtrip, HMAC verify),
then deterministic `[bench]` lines — epoch RK4 step ≈ 90,704 ticks,
Benettin Lyapunov step ≈ 104,461, HKDF key ≈ 3,743, AES-256-GCM(1024 B) ≈
41,757, HMAC(1040 B) ≈ 7,896, packet total ≈ 53,396 — and
`[done] all gates passed`. Units are guest instructions (QEMU-icount),
not measured hardware cycles; see `firmware/stm32f4-bench/bench_results.json`
and the Limitations hardware section.

### Hardware capture (physical STM32F4 Discovery, serial-free)

With an ST-LINK attached (WSL: `usbipd attach --wsl --busid <id>`):

```bash
make firmware-capture
```

Flashes the board, runs it, lets OpenOCD poll the SRAM2 done flag, halts,
dumps the bench log, and parses it (fails hard if the SysTick/DWT probes
disagree >5% or any gate fails). The board used for the archived
`bench_results.json` produced: RK4 step 685,378 cycles, packet total
421,176 cycles (2.51 ms), epoch maintenance 40.8% of the 1200 s epoch;
SysTick vs DWT probes 1,200,006 vs 1,200,011; two runs byte-identical.
Hardware values reproduce on the same board model only to ±cache/wait-state
jitter; the QEMU counts above are the deterministic reference.

## Cleanup

```bash
rm -rf results_fresh results_v3_repro
```

Removes fresh outputs. The canonical archive under `results_v3/` is never
overwritten by any reproduction command; direct runs default to
`--results-dir results` only when the flag is omitted — always pass an
explicit fresh directory when re-running.
