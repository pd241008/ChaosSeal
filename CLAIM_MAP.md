# Scientific Claim Map

Stable identifiers for every scientific claim in the artifact. Each entry
links a paper claim to its evidence, generating script, and output file.
All paths are relative to the repository root.

| ID | Paper Claim | Manifest | Evidence | Script | Output |
|---|---|---|---|---|---|
| C1 | Chaos-derived key rotation costs no goodput versus a cheap HKDF counter baseline | `experiment_manifests/E-C1-ECOSYSTEM-SWEEP.json` | Pendulum and counter goodput differ by <1% at every `|R|` ∈ {1..512}; both fall below BPSec past the crossover (~`|R|`=64–128, bootstrap 95% CI below) | `run_sweep_v3.sh` (Tier 1a) | `results_v3/v3-rsweep-seed*-r*.json` (50 runs), `results_v3/v3_sweep_stats.csv` |
| C2 | The chaotic pendulum provides dynamical entropy amplification that a counter cannot replicate | `experiment_manifests/E-C2-CORRUPTION-TEST.json` | 100% of 256 single-bit IC corruptions produce a diverged session key; mean divergence 15.9 Lyapunov time-scales ≈ 10.7 s. Counter detects at epoch 0 but with no amplification | `netsim_v2` `--corruption-test` | `results_v3/v3-corruption-test.json`, `results_v3/figures/v3_corruption_divergence.pdf` |
| C3 | The bounded wrapped coupling is a robustly chaotic, bounded attractor (λ₁ ≈ 0.405 nats/s, min 0.379) | `experiment_manifests/E-C3-LAMBDA-ROBUSTNESS.json` | λ_min series over 10×1000-draw attractor trials; parameter robustness sweep with honest weak bands (c<0.35, mass=0.5, length=4.0) | `scripts/regen_lambda_min_series.py`, `scripts/regen_robustness.py` | `results_v3/v4_lambda_min_series.csv`, `results_v3/pendulum_robustness_sweep.csv` |
| C4 | The full Lyapunov spectrum (KS entropy) converges at finite horizon and widens the entropy bound 2.56× over λ₁ alone | `experiment_manifests/E-C4-SPECTRUM-CONVERGENCE.json` | Relative change 2000s→4000s: λ₁ 1.31%, λ₃ 3.91%, KS 2.76%; KS bound 179.5 s vs λ₁ bound 460.0 s (256-bit) | `scripts/verify_spectrum_convergence.py` | `results_v3/convergence/spectrum_T*.csv`, `results_v3/convergence/convergence_summary.csv` |
| C5 | Burst loss inflates the effective HMAC verification interval but does not materially degrade goodput | `experiment_manifests/E-C5-COMMIT-LOSS.json` | Under all five Gilbert-Elliott profiles (0–55% stationary loss), absolute goodput change <0.01%; effective-N inflation up to 2.2× at N=1 | `analysis/commit_loss_sweep.py` | `results_v3/v3_commit_loss_sweep_stats.csv`, `results_v3/figures/v3_goodput_vs_commit_interval_loss.pdf` |
| C6 | The goodput crossover point is statistically characterized (bootstrap CI) | `experiment_manifests/E-C6-CROSSOVER-CI.json` | 10,000-resample bootstrap over 5 seeds: 95% CI [0.14%, 8.34%] of N; the wide CI is driven by seed 4's outlier — excluding it gives [8.15%, 8.34%] | `analysis/bootstrap_crossover.py` | `results_v3/v3_bootstrap_crossover.csv`, `results_v3/v3_crossover_bootstrap_stats.csv` |
| C7 | Dynamic membership (joins) costs no more than revocation and is practically free | `experiment_manifests/E-C7-JOIN-PROTOCOL.json` | Join broadcast = revocation broadcast (covering-set bound, 2048 B at N=1024, r=8); worst-case 1 join/10 s with 60 s rebuild consumes ~0.0005% of epoch downlink | `core_v2` `cli_v2 join-protocol`, `netsim_v2` `--membership-test` | `results_v3/v3-membership-join-n1024-r8.json`, `results_v3/v3-membership-join-n4096-r8.json` |
| C8 | Single-bit IC corruption detection and resync behavior (counter vs chaos) | `experiment_manifests/E-C8-RESYNC-SENSITIVITY.json` | Per-packet-HMAC resync recovers independently per lost/accepted packet; resync fraction scales ~linearly with loss (≈9% @1%, ≈99% @10%) for both key sources | `analysis/v3_analysis.py` (data_stream_loss_sensitivity) | `results_v3/v3_loss_sweep_stats.csv`, `results_v3/v3_loss_sweep_*.json` |
| C9 | Goodput scales ~linearly with payload; chaosseal ≈ counter and modestly above BPSec | `experiment_manifests/E-C9-SIZE-SWEEP.json` | 128 B–4096 B payload sweep, R=8: chaosseal within noise of counter at every size; both above BPSec at moderate-to-large payloads | `run_sweep_v3.sh` (Tier 2b) | `results_v3/v3-size-sweep-*.json`, `results_v3/v3_size_sweep_stats.csv` |
| C10 | The pendulum key-schedule output has no exploitable low-entropy structure vs a counter input | `experiment_manifests/E-C10-KEYSTREAM-ENTROPY.json` | Keystream entropy comparison of chaotic vs counter HKDF input material (generalization analysis v4) | `analysis/v4_generalization.py` | `results_v3/figures/v4_keystream_entropy.pdf`, `results_v3/figures/v4_lambda_min_distribution.pdf`, `results_v3/figures/v4_crossover_surface.pdf` |

## Claim Dependencies

```
C3 (bounded chaotic attractor established)
    └── enables C4 (finite-horizon spectrum convergence)
          └── strengthens the epoch-bound argument (docs/end_to_end_security_and_membership.md Part A.3)
C1 (ecosystem sweep)
    ├── motivated C6 (crossover CI: how sharp is the crossover?)
    └── motivated C2 (what does the pendulum buy over a counter?)
          └── confirmed by C8 (resync sensitivity) and C10 (keystream entropy)
C7 (join protocol) mirrors the revocation cost used by C1
C5 (commit interval × loss) qualifies the HMAC commitment assumptions in C2/C8
C9 (payload scaling) is the robustness check on C1's absolute goodput numbers
```

## Cross-References

- **Threat model & protocol**: `docs/architecture.md`, `README.md` (abstract)
- **Formal security arguments**: `docs/end_to_end_security_and_membership.md`
- **Entropy-claim caveat**: `docs/design_note_metastability.md`, `README.md` caveat block
- **ADRs**: `docs/01-documentation/adrs/`
- **Postmortem (why the old numbers were retracted)**: `docs/02-postmortems/2026-09-06-metastable-pendulum-saturation-artifact.md`
- **Provenance**: `PROVENANCE.md`
- **Limitations**: `LIMITATIONS.md`
