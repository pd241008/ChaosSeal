# Reviewer Verification Checklist

Use this checklist to verify the artifact systematically. Each section maps
to a claim or reproducibility requirement.

## Environment Setup
- [ ] Rust ≥ 1.75, Go ≥ 1.22, Python ≥ 3.10 available
- [ ] `core_v2` builds clean (`cargo build --release`)
- [ ] Rust tests pass: 9 lib + 14 KAT (`cargo test --release`)
- [ ] `netsim_v2` builds and tests green (`go vet ./... && go test ./...`)
- [ ] `pip install -r requirements.txt` succeeds (numpy, matplotlib)
- [ ] *(optional)* `firmware/stm32f4-bench` builds for `thumbv7em-none-eabihf`; with `qemu-system-arm` installed, `make firmware-bench` passes all three on-target gates and ends `[done] all gates passed`

## Claim C1 — Chaos key rotation costs no goodput
- [ ] `results_v3/v3-rsweep-seed*-r*.json` present (50 files, 5 seeds × 10 R)
- [ ] `python3 analysis/v3_analysis.py` regenerates `results_v3/v3_sweep_stats.csv`
- [ ] chaosseal vs counter |Δ goodput| < 1% at every R
- [ ] BPSec ≈ 1.5978 Mbps flat; crossover located between R=64 and R=128

## Claim C2 — Dynamical entropy amplification
- [ ] `results_v3/v3-corruption-test.json` present with 256 bit positions
- [ ] `chaos_key_differs_fraction` = 1.0
- [ ] `chaos_mean_divergence_lyap_timescales` ≈ 15.9 (≈ 10.7 s)
- [ ] `counter_mean_epochs_to_detect` = 0 (no amplification, by design)

## Claim C3 — Bounded chaotic attractor (λ₁ ≈ 0.405)
- [ ] `python3 scripts/validate_benettin.py` prints `ALL MATCH`
- [ ] `results_v3/v4_lambda_min_series.csv` present (10 trials × 1000 draws)
- [ ] `results_v3/pendulum_robustness_sweep.csv` present with weak bands identified
- [ ] `results_v3/figures/v4_pendulum_robustness.pdf` renders the sweep

## Claim C4 — Full Lyapunov spectrum convergence
- [ ] `results_v3/convergence/spectrum_T{200,500,1000,2000,4000}s.csv` present
- [ ] `results_v3/convergence/convergence_summary.csv` matches the paper table
- [ ] λ₁ relative change 2000s→4000s ≈ 1.31%; KS ≈ 2.76%
- [ ] KS entropy bound (179.5 s for 256-bit) vs λ₁ bound (460.0 s) = 2.56× margin
- [ ] (optional) `python3 scripts/verify_spectrum_convergence.py` re-runs all horizons

## Claim C5 — Commitment interval under LEO loss profiles
- [ ] `python3 analysis/commit_loss_sweep.py` executes without error
- [ ] `results_v3/v3_commit_loss_sweep_stats.csv` covers 5 profiles × 5 N values
- [ ] Absolute goodput degradation < 0.01% under every profile
- [ ] Effective-N inflation ~2.2× at N=1 under the extreme (55%) profile

## Claim C6 — Bootstrap CI on the crossover
- [ ] `python3 analysis/bootstrap_crossover.py` executes without error
- [ ] `results_v3/v3_crossover_bootstrap_stats.csv` reports 95% CI [0.14%, 8.34%]
- [ ] Seed-4 outlier identified (low goodput at R=2)
- [ ] Excluding seed 4: tight CI [8.15%, 8.34%]

## Claim C7 — Join protocol cost
- [ ] `cli_v2 join-protocol --n 1024 --r 8 --joins 1 --rebuild-interval-epochs 1.0 --epoch-duration-s 1200` runs
- [ ] Join broadcast = 2048 B (= revocation cost) at N=1024, r=8
- [ ] `results_v3/v3-membership-join-n1024-r8.json` and `-n4096-r8.json` present
- [ ] Worst-case row (1 join/10 s, 60 s rebuild) ≈ 0.0005% of epoch capacity

## Claim C8 — Resync sensitivity
- [ ] `results_v3/v3_loss_sweep_stats.csv` present
- [ ] Data-stream resync fraction scales ~linearly with loss (≈9% @1% → ≈99% @10%)
- [ ] Behavior identical for chaosseal and counter

## Claim C9 — Payload scaling
- [ ] `results_v3/v3_size_sweep_stats.csv` covers 128–4096 B
- [ ] Goodput scales ~linearly with payload
- [ ] chaosseal ≈ counter at every size; both above BPSec at ≥512 B

## Claim C10 — Keystream entropy
- [ ] `python3 analysis/v4_generalization.py` executes
- [ ] `results_v3/figures/v4_keystream_entropy.pdf` produced
- [ ] `results_v3/figures/v4_lambda_min_distribution.pdf` and `v4_crossover_surface.pdf` produced

## Provenance
- [ ] `PROVENANCE.md` documents the legacy→v2→v3 evolution and retraction
- [ ] `docs/02-postmortems/2026-09-06-metastable-pendulum-saturation-artifact.md` read
- [ ] `docs/01-documentation/adrs/` contains ADR-001 through ADR-004
- [ ] Legacy `core/`/`netsim/` clearly distinguished from canonical `core_v2`/`netsim_v2`

## Limitations
- [ ] Verifier-gated entropy numbers understood (README caveat block)
- [ ] Simulator-model limitations understood (24-sat visibility heuristic, wall-clock timing)
- [ ] Fixed-point saturation floor and large-deviation caveats understood
- [ ] No hardware benchmark (STM32/Cortex-M4) — code-side work complete only

## Artifact Integrity
- [ ] Every `results_v3/*.json` embeds `git_commit`, `rng_seed`, `parameters`
- [ ] Canonical archive is never overwritten by reproduction commands
- [ ] `docs/v3_results.md` numbers trace to `analysis/v3_analysis.py` output
- [ ] No author-identifying information in committed files beyond the public README citation block
