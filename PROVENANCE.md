# Provenance

This document records the methodological evolution of the evaluation
pipeline. Superseded implementations are retained for scientific provenance
and should not be interpreted as the canonical evaluation pipeline.

## Evolution Timeline

```
Legacy Generation (core/ + netsim/, v1)
       │
       ├── Q32.32 core, KATs, C ABI, CLI
       ├── netsim with TLS 1.3 + BPSec baselines
       ├── results/ (manual.json), results_v2/ (first R-sweep)
       ▼
Metastability Investigation
       │
       ├── Linear elastic coupling found UNBOUNDED (energy escape)
       ├── Q32.32 long-horizon "attractor" identified as
       │   ±2^31 saturation artifact ("phantom attractor")
       ├── RETRACTED: 840.6 s epoch bound (measured from the
       │   unbounded linear-coupling pendulum)
       ├── ADR-001 (Benettin tangent fix), ADR-002 (Jacobian inertia fix)
       ├── ADR-003 (claim reframe, verify-before-trust gating)
       ▼
Bounded Wrapped Coupling Redesign (v2)
       │
       ├── ADR-004: atan2(sin Δθ, cos Δθ) sawtooth spring, default c=1.0
       ├── core_v2/ + netsim_v2/ (canonical generation)
       ├── Float64 replicator validation → ALL MATCH
       ▼
V3 Multi-Tier Strengthening (canonical)
       │
       ├── Counter-mode HKDF baseline (ecosystem sweep)
       ├── Packet-size / loss / commitment-interval sweeps
       ├── Single-bit corruption sensitivity test
       ├── Full Lyapunov spectrum + finite-horizon convergence
       ├── Commit-interval × LEO loss profiles (Gilbert-Elliott)
       ├── Bootstrap CI on the crossover point
       ├── Join protocol (dynamic membership) cost model
       └── End-to-end security composition (formal arguments)
```

## Retracted Numbers

| Number | Origin | Status |
|---|---|---|
| "840.6 s epoch bound" (abstract) | Measured from the *linear*-coupling pendulum | **Retracted 2026-09-06** — the linear-coupling model is not a bounded chaotic attractor; the number does not describe a physical 1200 s epoch |
| λ₁ ≈ 1.46, KS ≈ 2.9 nats/s (T=8000 s) | Q32.32 long-horizon integration | **Retracted** — exponent of the *saturated* system (±2^31), not the ODE |
| T=100 s robustness-sweep rates | Pre-redesign committed data | **Tagged transient** — internally consistent and cross-validated, but describes the bounded-swing transient of an unbounded model |
| "~6% crossover" (paper prose) | Early single-seed reading | **Superseded** by the measured 5-seed mean 8.3% with bootstrap CI (`results_v3/v3_crossover_bootstrap_stats.csv`) |

## Superseded Components

### `core/` + `netsim/` (legacy generation)

**Original role**: First complete implementation (Deliverables 1–3): Q32.32
core with KATs, Go netsim with TLS 1.3 and BPSec baselines, Python analysis
(`analysis/stats.py`, `analysis/figures.py`, `analysis/Makefile`).

**Why superseded**: Implements the linear elastic coupling and the original
Lyapunov estimator path; predates the metastability redesign. The v2
generation (`core_v2/`, `netsim_v2/`) implements the bounded wrapped coupling
and the corrected Benettin tangent update.

**Archive**: Retained for provenance. `scripts/legacy/run_sweep.sh` and `scripts/legacy/run_sweep_v2.sh`
drive this generation; `results/` and `results_v2/` hold its outputs. No
canonical claim uses them.

### `analysis/stats.py` + `analysis/figures.py` (legacy analysis)

**Original role**: Paper figures and statistics over `results/*.json`
(v1 schema, 3 figures: BEE size vs R, resync latency, throughput).

**Why superseded**: Reads the v1 results schema only. V3 aggregation lives in
`analysis/v3_analysis.py` over `results_v3/`.

**Archive**: Retained; `analysis/README.md` documents the original metric
definitions (still the definitional reference for goodput/latency).

## Canonical Components

| Component | Role |
|---|---|
| `core_v2/` | Protocol engine: Q32.32 RK4, bounded wrapped coupling (c=1.0), Benettin λ + full spectrum + KS, HKDF→AES-256-CTR/GCM, HMAC-SHA256, BEE key tree, join protocol |
| `netsim_v2/` | LEO simulation: visibility windows, elevation-dependent latency, Gilbert-Elliott bursts; chaosseal/counter/BPSec baselines; corruption + membership experiments |
| `scripts/run_sweep_v3.sh` | Canonical pipeline driver (all tiers) |
| `analysis/v3_analysis.py` | Canonical aggregation (stats CSVs + figures) |
| `analysis/{commit_loss_sweep,bootstrap_crossover,v4_generalization,burst_model}.py` | Claim-specific analyses (C5, C6, C10/C3, Figure 7) |
| `scripts/verify_*.py`, `scripts/validate_benettin.py` | Independent verification gates |
| `scripts/regen_*.py`, `scripts/sample_lyapunov.py`, `scripts/reference_spectrum_long_horizon.py` | Data regeneration utilities |
| `results_v3/` | Canonical archive (68 run JSONs + CSVs + 13 figures) |
| `firmware/stm32f4-bench/` | Cortex-M4 no_std benchmark: core_v2 sources vendored verbatim (deltas documented per file), crypto pinned to `core_v2/Cargo.lock`; hardware-measured results (STM32F4 Discovery, 2026-09-14, SysTick+DWT dual-clock validated) and QEMU-icount reference in `bench_results.json` |

## Decision Records

See `docs/README.md` and `docs/01-documentation/adrs/` for the full ADR
history:

- **ADR-001**: Correct Benettin tangent update (linearized Jacobian flow, not constant matrix)
- **ADR-002**: Jacobian inertia placement (damping outside /I, coupling inside /I)
- **ADR-003**: Entropy-claim reframe on metastability; verify-before-trust gating
- **ADR-004**: Bounded wrapped coupling redesign, default c=1.0

Postmortem: `docs/02-postmortems/2026-09-06-metastable-pendulum-saturation-artifact.md`
documents the full failure chronology, including the vectorized-batch `deriv`
bug that nearly produced a false negative during the redesign verification.
