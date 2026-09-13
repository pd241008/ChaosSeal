# Known Limitations

This artifact is a research prototype. It does not provide production-grade
guarantees. Reviewers should understand these boundaries before interpreting
results.

## Verifier-Gated Entropy Numbers

**Status**: Open — the headline gating condition of the artifact.

**Description**: All security numbers derived from the pendulum (λ₁ ≈ 0.405,
KS ≈ 1.0–1.3 nats/s, 256-bit dt ≈ 136–176 s design-stage; spectrum-based KS
bound 179.5 s) are **design-stage measurements**, not final security claims.
They require independent verifier confirmation and carry two standing
caveats: the sampled λ_min is not a proven global minimum, and the
fixed-point discretization floor is unbounded above.

**Impact**:
- Manuscript numbers remain gated per the README caveat block (2026-09-06).
- The abstract's original "840.6 s epoch bound" is **retracted** (see
  `PROVENANCE.md` and the postmortem).

**Mitigation**: `scripts/validate_benettin.py` (float64 referee) and
`scripts/verify_metastability.py` encode the verify-before-trust discipline;
all committed rate data is tagged with its provenance and horizon.

## Fixed-Point Saturation Floor

**Status**: Inherent limitation, understood.

**Description**: Q32.32 states saturate at ±2^31. Long-horizon claims require
a bound on the discretization floor; a bounded-looking trajectory under
saturation is not evidence of a bounded system (this exact failure produced
the retracted "phantom attractor").

**Impact**: Any long-horizon fixed-point statistic is a fidelity ceiling, not
a confinement argument.

**Mitigation**: The bounded wrapped coupling is bounded *by construction*
(`atan2(sin Δθ, cos Δθ)`, ADR-004), independent of the fixed-point floor;
three dedicated wrapped-coupling tests (Jacobian slope-1 across branches,
on-cut behavior, spin-boundedness) run in `cargo test --release`.

## Simulator Visibility Model

**Status**: Known open issue.

**Description**: The 24-satellite / 600 s window yields `visible_pct` ≈ 0.7–2%
in v2/v3 runs, which is suspiciously low for a real 24-sat LEO constellation
(flagged in `docs/PROGRESS.md`). Baselines transmit at the strongest-link
moment rather than a fixed offset.

**Impact**: Absolute goodput values depend on this visibility heuristic;
comparisons *between* baselines under the same link model are unaffected.

**Mitigation**: Claims are stated comparatively (chaosseal vs counter vs
BPSec under identical links), never on absolute Mbps against external systems.

## Wall-Clock Timing Noise

**Status**: Inherent limitation.

**Description**: The simulator records real wall-clock crypto timing.
Run-to-run jitter of ~0.1–1% in goodput is observed, with isolated outliers
(one 3% outlier of a 490 µs crypto measurement in Tier 2a).

**Impact**: R3-level quantities (see `REPRODUCIBILITY_LEVELS.md`) reproduce
within noise, not exactly.

**Mitigation**: Multi-seed reporting (5 seeds) and mean±std tables; all
claims are stated on differences exceeding the noise floor.

## BEE Covering-Set Simplification

**Status**: Known limitation.

**Description**: The BEE covering-set size uses a simplified heuristic
(`c(N,r) = ⌈log₂N / log₂ r⌉ · r` for r ≥ 2), not a full subset-difference
implementation (flagged in `docs/PROGRESS.md`).

**Impact**: Absolute broadcast sizes (2048 B at N=1024, r=8) may differ from
a complete Complete Subtree / Subset Difference scheme; the *comparative*
structure (join cost = revocation cost) is unaffected.

**Mitigation**: Stated explicitly in `docs/end_to_end_security_and_membership.md`
Part B; the join-vs-revoke equivalence follows from the construction, not the
constant factors.

## No Hardware Benchmark

**Status**: Open — the only remaining Future Work item.

**Description**: The STM32/Cortex-M4 (or ESP32) benchmark of the Q32.32 RK4 +
HKDF + AES-GCM path has not been run. Porting `core_v2/src/kinematics/` and
`core_v2/src/crypto/` to `no_std` + `cortex-m-rt` is required, plus flashing
to real hardware (or QEMU) and measuring cycles-per-epoch and per-packet
AEAD cost.

**Impact**: Cross-platform timing claims are supported by Q32.32 determinism
arguments, not by measured hardware cycles.

**Mitigation**: All code-side work is complete and reproducible; only the
hardware measurement remains.

## Randomness and Reproducibility Boundary

**Status**: Design boundary.

**Description**: Netsim runs are exactly reproducible only given the same
seed, flags, and git commit (R2). Wall-clock-derived quantities never
reproduce bit-exactly (R3). There is no unseeded/stochastic component in the
canonical pipeline; all randomness derives from `--seed`.

**Impact**: Reviewers must not expect archived JSONs to regenerate
byte-identically from a replay unless the commit matches.

**Mitigation**: `REPRODUCIBILITY_LEVELS.md` classifies every result class;
each archived JSON embeds `git_commit`, `rng_seed`, and `parameters`.
