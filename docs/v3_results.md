# Tier 1–3 Evaluation Results (Counter Baseline, Sweeps, Corruption, Proofs)

This note summarizes the measured results from the multi-tier strengthening of the
ChaosSeal/CEP evaluation. Raw data is in `results_v3/`, aggregation script in
`analysis/v3_analysis.py`, and the formal arguments in
`docs/end_to_end_security_and_membership.md`. All numbers below are real
measurements from `results_v3/`.

---

## Tier 1a — Goodput: Pendulum (chaosseal) vs Counter baseline vs BPSec
N=1024, epoch = 1200 s, R ∈ {1…512}, 5 seeds. Mean ± std over 5 seeds (Mbps).

| R   | chaosseal | counter  | bpsec    | Δ(chaos vs counter) |
|-----|-----------|----------|----------|---------------------|
| 1   | 1.6446    | 1.6359   | 1.5978   | +0.5% |
| 8   | 1.6435    | 1.6540   | 1.5978   | −0.6% |
| 32  | 1.6363    | 1.6478   | 1.5978   | −0.7% |
| 64  | 1.6221    | 1.6264   | 1.5978   | −0.3% |
| 128 | 1.5465    | 1.5512   | 1.5978   | −0.3% |
| 256 | 1.3045    | 1.3050   | 1.5978   | −0.04% |
| 512 | 0.8019    | 0.8036   | 1.5978   | −0.2% |

**Finding.** The pendulum and counter baselines are *indistinguishable* in goodput
(differ by <1% at every R). Both fall below BPSec as R grows (crossover ~R=64).
The chaotic key-derivation therefore costs **no measurable goodput penalty** versus
the cheap HKDF counter — its benefit is purely the entropy/forward-hiding property,
not throughput.

## Tier 1b — Single-bit corruption detection (256 bit positions)
Counter: detection epoch = **0 for all positions** (immediate bit-level sensitivity,
no dynamic amplification). Chaos: **100% of positions** produce a diverged session
key; mean divergence 15.9 Lyapunov time-scales ≈ 10.7 s (range 0.01–50.9 s).
See `results_v3/v3-corruption-test.json` and
`results_v3/figures/v3_corruption_divergence.pdf`.

## Tier 2a — Packet-loss sweep (R=8)
Single-clean-packet goodput is loss-invariant (0.1–1% spread = wall-clock noise;
one 3% outlier of 490 µs crypto wall-clock). The meaningful loss model is
`data_stream_loss_sensitivity` over 10,000 packets: resyncs scale ~linearly with
loss rate (≈9% at 1%, ≈30% at 3%, ≈48% at 5%, ≈99% at 10%) for both chaosseal and
counter, confirming the per-packet-HMAC resync recovers from each lost/accepted
packet independently.

## Tier 2b — Packet-size sweep (R=8, 1024 B reference)
| size | chaosseal | counter | bpsec |
|------|-----------|---------|-------|
| 128  | 0.212     | 0.212   | 0.212 |
| 512  | 0.837     | 0.841   | 0.825 |
| 1024 | 1.646     | 1.651   | 1.598 |
| 4096 | 5.991     | 6.006   | 5.363 |

Goodput scales ~linearly with payload; chaosseal ≈ counter throughout, modestly
above bpsec at moderate-to-large payloads.

## Tier 2c — Commitment interval N (chaosseal)
| N | overhead% | goodput Mbps |
|---|-----------|--------------|
| 1   | 6.250 | 1.643 |
| 16  | 3.320 | 1.643 |
| 256 | 3.137 | 1.649 |
| 1024| 3.128 | 1.649 |

Overhead per HMAC verify = 32/N bytes; goodput impact is essentially flat above
N=16 (diminishing returns past N≈64). Figure:
`results_v3/figures/v3_goodput_vs_commit_interval.pdf`.

## Tier 3
`docs/end_to_end_security_and_membership.md` contains (A) an explicit hybrid-argument
composition of the BEE collusion-resistance bound with the data-layer AEAD
IND-CCA2 argument into a single end-to-end adversarial bound, and (B) a dynamic
membership rebuild-cost bound showing a join costs no more than a revocation
(2048 B at N=1024, r=8), with a worked amortized-vs-latency table grounded in the
measured ~247 MB/epoch capacity.

---

## Artifacts
- `results_v3/*.json` — 68 raw run outputs.
- `results_v3/v3_sweep_stats.csv`, `v3_commit_sweep_stats.csv` — aggregated tables.
- `results_v3/figures/*.pdf` — 4 figures (goodput-vs-R, vs-size, vs-commit-interval,
  corruption divergence).
- `analysis/v3_analysis.py` — reproducible aggregation.
- `docs/end_to_end_security_and_membership.md` — Tier 3 formal arguments.
