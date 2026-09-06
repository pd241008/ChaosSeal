# ADR 003: Reframe the 256-bit/epoch Entropy Claim — Pendulum Becomes a Transient Chaotic Conditioner

## Status
Accepted (2026-09-02)

## Context
The manuscript claimed a 256-bit-per-epoch entropy accumulation from the
reinjected 3-pendulum ODE: `dt_bound = 256 ln2 / lambda_min` was quoted at
232–282 s (and 6932 s in a secondary row), with expected phase-space
reconstruction attacks neutralized by HKDF+AES rather than raw keystream.

Investigation (`scripts/verify_metastability.py`, all-integrator,
all-inertia) established:

- The **linear elastic coupling** `c * (theta_i - theta_prev)/d * 0.1` is a
  globally unbounded parabolic potential. After kick episodes the relative
  angle ramps without bound and the coupling pumps kinetic energy in a
  deterministic escape: |omega| blows up ~18 s (worst-case random IC) to
  ~127 s (deterministic cold-start IC) across RK4 (dt=1e-2, 2e-3), symplectic
  Verlet, and DOP853.
- **Fixed-point Q32.32 "long-horizon attractors" are saturation artifacts.**
  Every state saturates at ±2^31 via saturating i64 arithmetic, manufacturing
  a bounded-looking trajectory whose converged spectrum (λ1 ≈ 1.46, KS ≈ 2.9
  nats/s at T=8000 s) is the exponent of the **saturated** system, not the ODE.
- Kolmogorov–Sinai entropy is therefore **not defined** for the committed
  model at the protocol epoch: there is no attractor over which to average.

Committing the numbers as-is would have been a genuine leak of the
honesty-first policy (`analysis/research-patterns.md`, ChaosSeal §1): the 
T=100 s-window data *was* internally consistent and cross-validated, but it
described the bounded-swing transient of an unbounded model — extrapolating it
to a 1200 s epoch is invalid.

## Decision
1. **Reframe the claim.** Do not present any 256-bit/epoch export; document the
   pendulum as a transient chaotic *conditioner* of hardware-supplied physical
   noise, contributing forward-hiding but not a self-contained entropy source.
2. **Tag the committed T=100 s data** as bounded-swing-transient statistics
   (robustness sweep, λ_min series, validate_benettin matches), reproducible
   but not extrapolatable to the protocol epoch.
3. **Verify-before-trust gate.** No manuscript figure or final security number
   may change until an independent verifier reproduces the finding from first
   principles (`verify_metastability.py`, retained as the gate).
4. Keep `regen_lambda_min_series.py`'s committed random draws untouched so the
   historical artifact chain remains reproducible.

Commits: `4c3d017` (reframe + verification + epoch-cap comparison data),
`1cc6726` (bounded-coupling exploration), which begot ADR-004.

## Reasoning
A paper that keeps a headline number after the model behind it is discovered
non-integrable trades short-term credibility for long-term falsifiability. The
reframe is the cheaper option: it preserves the protocol architecture (BEE
revocation + HKDF keying + fixed-point determinism + HMAC commitment), the
netsim evaluation, and the honest parts of the entropy argument, while
surfacing the exact measurement that killed the original claim.

## Consequences
- **Good** 🟢: Every committed number now describes the model it was measured
  from. The conditioned-noise model is a conservative, defensible claim.
- **Bad** 🔴: A headline security property is demoted to a conditioning role;
  the 256-bit claim had to be re-earned by a different model (see ADR-004). The
  saturation-artifact realization means any future long-horizon number must
  first bound the fixed-point floor.

## Revisit When
- A *bounded* model variant with a cross-validated KS rate is adopted and
  verified (this is exactly what happened — see ADR-004), or
- The product hardens to accept conditioned-noise entropy only, in which case
  the reframe is permanent.