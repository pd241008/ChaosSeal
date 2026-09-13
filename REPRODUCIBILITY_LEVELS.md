# Reproducibility Levels

Not every result in this artifact has the same reproducibility guarantee.
This document classifies results into four levels so reviewers can
understand what to expect.

## Level Definitions

| Level | Meaning | Examples in this artifact |
|-------|---------|---------------------------|
| R1 | Exact deterministic reproduction (fixed IC / fixed seed) | Q32.32 RK4 kinematics, Lyapunov estimates at fixed ICs, BEE covering-set sizes, join-protocol cost model, KATs |
| R2 | Deterministic given recorded inputs (seed + git commit + flags) | Archived netsim JSON runs, aggregation tables, bootstrap CI, spectrum-convergence CSVs |
| R3 | Wall-clock / timing-dependent (reproduces within noise) | Goodput Mbps, overhead %, crypto wall-clock, resync fractions, loss/size/commit sweeps |
| R4 | Verification using archived canonical outputs | Paper figures, `docs/v3_results.md` numbers, keystream-entropy and crossover-surface figures |

## R1: Exact Deterministic

Fixed-point Q32.32 arithmetic has no floating-point divergence. Given the
same initial condition and parameters, the core reproduces bit-exactly,
on any platform.

- AES-256-CTR and HMAC-SHA256 KATs (RFC 3686 / RFC 4231 vectors) — `core_v2/tests/kat.rs`
- Determinism tests (same seed → bit-exact output) — `core_v2/src/lib.rs`
- Lyapunov estimates at a fixed IC — cross-checked by the independent
  float64 replicator `scripts/validate_benettin.py` (`ALL MATCH`)
- BEE covering-set size / join-protocol cost model —
  `cli_v2 beesize`, `cli_v2 join-protocol`

## R2: Deterministic Given Recorded Inputs

Every archived netsim run embeds `git_commit`, `rng_seed`, and the full
`parameters` block. Replaying the same flags on the same commit reproduces
the recorded event sequence and byte counts.

- `results_v3/*.json` (68 archived run outputs)
- Aggregation tables (`analysis/v3_analysis.py` output CSVs)
- Bootstrap CI (`analysis/bootstrap_crossover.py` seeds its resampler
  deterministically with `np.random.seed(42)`)
- Spectrum-convergence CSVs (`results_v3/convergence/`)

## R3: Wall-Clock / Timing-Dependent

The simulator records real network events and wall-clock crypto timing.
Derived goodput quantities reproduce within run-to-run noise.

- Goodput (Mbps) for all baselines — observed spread ~0.1–1%
- One 3% wall-clock outlier of 490 µs crypto timing documented in Tier 2a
- Resync fractions over the 10,000-packet data-stream sensitivity probe
- Commitment-interval overhead percentages

Mitigation: multi-seed reporting (5 seeds for the R-sweep) and mean±std
tables; claims are stated on differences that exceed the noise floor
(e.g. chaosseal vs counter <1% everywhere, which is itself the finding).

## R4: Archival Verification

These outputs are generated from the canonical result files and should be
verified by regenerating statistics from the archive, not by re-running
simulations.

- `results_v3/figures/*.pdf` (13 figures)
- `docs/v3_results.md` (all quoted numbers trace to `analysis/v3_analysis.py`
  output over `results_v3/*.json`)
- `results_v3/figures/v4_*.pdf` (generalization analysis from
  `analysis/v4_generalization.py`)

## Implications for Reviewers

- Do not assume R3 results are bit-identical across runs. Compare against
  the noise floor stated in each claim.
- R1 can be verified with `cargo test --release` and the CLI cost commands.
- R2 can be verified by replaying recorded flags (see `REPRODUCE.md`).
- R4 verification requires reading the canonical results and confirming
  they support the paper's tables and figures.
