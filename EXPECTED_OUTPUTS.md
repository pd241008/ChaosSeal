# Expected Outputs

This document describes what happens when you run each major experiment.
Use this to verify that your environment is correctly configured.

## Core Build and Tests

```bash
cd core_v2 && cargo build --release && cargo test --release && cd ..
```

**Expected runtime**: <1 min

**Expected output**:
```
test result: ok. 9 passed; 0 failed        (lib unittests)
test result: ok. 14 passed; 0 failed       (KATs: RFC 3686 AES-CTR, RFC 4231 HMAC-SHA256)
```

**Expected properties**: build clean, no warnings; all wrapped-coupling and
determinism tests green.

## Simulator Build and Tests

```bash
cd netsim_v2 && CGO_ENABLED=1 go build -o chaoseal-sim . && go vet ./... && go test ./... && cd ..
```

**Expected runtime**: <1 min

**Expected output**: `go vet` clean; `ok` for `core/client`, `core/crypto`,
`core/engine`, `core/kinematics`, and the `test/` integration suite.

## Single Simulation Run

```bash
./netsim_v2/chaoseal-sim --run-id smoke --seed 1 --bee-r 8 \
  --baselines chaosseal,counter,bpsec \
  --results-dir results_fresh --rust-cli "$(pwd)/core_v2/target/release/chaosseal"
```

**Expected runtime**: <1 s

**Expected output**:
```
run_id=smoke satellites=24 baselines=[chaosseal counter bpsec]
link: visible=... mean_latency=...ms loss=...
baseline bpsec: ok
baseline chaosseal: ok
baseline counter: ok
results -> <abs path>/results_fresh/smoke.json
```

**Expected properties**: exactly one JSON file with `git_commit`, `rng_seed`,
`parameters`, `link_stats`, and per-baseline results.

## Tier 1a — R-Sweep (50 runs)

```bash
./run_sweep_v3.sh results_v3_repro
```

**Expected runtime**: minutes (each run ≈ 0.3 s, plus builds)

**Expected files**: `results_v3_repro/v3-rsweep-seed{1..5}-r{0001..0512}.json`
plus all Tier 2 JSONs and the corruption test.

**Expected properties** (aggregate via `python3 analysis/v3_analysis.py`):
- chaosseal and counter goodput differ by **<1% at every R**
- BPSec flat ≈ 1.5978 Mbps; both chaos baselines fall below it past the
  crossover (~R=64–128; bootstrap CI in `results_v3/v3_crossover_bootstrap_stats.csv`)

## Single-Bit Corruption Test

```bash
./netsim_v2/chaoseal-sim --run-id corruption --corruption-test \
  --results-dir results_fresh --rust-cli "$(pwd)/core_v2/target/release/chaosseal"
```

**Expected runtime**: seconds

**Expected properties**:
- `counter_mean_epochs_to_detect` = 0 (immediate bit-level sensitivity)
- `chaos_key_differs_fraction` = 1.0 (all 256 positions diverge)
- `chaos_mean_divergence_lyap_timescales` ≈ 15.9 (≈ 10.7 s)

## Lyapunov Spectrum

```bash
./core_v2/target/release/cli_v2 lyapunov-spectrum --pendulums 3 --mass 1.0 \
  --length 1.0 --damping 0.1 --coupling 1.0 --steps 20000 --samples 20
```

**Expected runtime**: ~1.5 s

**Expected output**: JSON with `success: true` and `output` containing
`lambda1_mean`, `lambda2_mean`, `lambda3_mean`, `ks_mean/min/max`,
`dt_bound_from_ks_min_s`, `dt_bound_from_l1_min_s`, and `raw_spectra`.

**Expected properties** (T=4000 s, 100 samples — archived in
`results_v3/convergence/`): λ₁ mean ≈ 0.408 (min 0.386), KS mean ≈ 1.34
(min 0.99), λ₁/λ₃/KS relative change <5% between the 2000 s and 4000 s
horizons.

## Verification Gates

```bash
python3 scripts/validate_benettin.py
```

**Expected output**: per-config `MATCH` lines ending in `ALL MATCH`
(float64 replicator vs Rust Q32.32). One non-gated
`BOUNDARY-DIVERGENCE (documented, not gated)` line may appear for a
boundary configuration.

```bash
python3 scripts/verify_metastability.py
```

**Expected output**: coupled-system escape vs c=0 control across integrators
(RK4 default; `--fine` adds dt=1e-4, `--dop853` adds scipy DOP853), plus the
model-bounds summary. This documents the *historical* linear-coupling finding.

## Commit Interval × Loss Sweep

```bash
python3 analysis/commit_loss_sweep.py
```

**Expected runtime**: seconds

**Expected files**: `results_v3/v3_commit_loss_sweep_stats.csv`,
`results_v3/figures/v3_goodput_vs_commit_interval_loss.pdf`,
`v3_commit_loss_degradation.pdf`, `v3_effective_commit_interval.pdf`

**Expected properties**: effective HMAC interval inflates up to ~2.2× at N=1
under the extreme (55%) profile, but absolute goodput change stays <0.01%
under all five profiles.

## Bootstrap CI on the Crossover

```bash
python3 analysis/bootstrap_crossover.py
```

**Expected runtime**: seconds

**Expected files**: `results_v3/v3_bootstrap_crossover.csv`,
`results_v3/v3_crossover_bootstrap_stats.csv`

**Expected properties**: 95% CI [1.4, 85.4] revoked (0.14%–8.34% of N=1024);
seed-4 outlier identified and excluded variant gives tight CI [83.5, 85.4]
(8.15%–8.34%).

## Aggregation and Figures

```bash
python3 analysis/v3_analysis.py
```

**Expected runtime**: seconds

**Expected files**: the five `results_v3/v3_*_stats.csv` tables and the
figures under `results_v3/figures/` (goodput-vs-R, vs-size,
vs-commit-interval, corruption divergence).

**Expected properties**: numbers match `docs/v3_results.md` exactly (they are
derived from the same archive).
