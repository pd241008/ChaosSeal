# Postmortem: The Metastable Pendulum and the Saturated "Attractor"

> **Date:** September 6, 2026
> **Severity:** High — a headline security claim was undefinable until redesigned
> **Status:** Complete — reframed (ADR-003), root cause fixed (ADR-004), re-validated

## Summary

The 3-pendulum oscillator at the heart of ChaosSeal's entropy argument was
claimed to accumulate **256 bits per epoch** from its measured Lyapunov rate.
Two independent flaws invalidated that claim, and a third nearly destroyed the
measurement itself:

1. **The elastic coupling was unbounded.** The linear term `c*(theta_i -
   theta_prev)/d*0.1` is a global parabola. After kick episodes the relative
   angle ramps without bound and the coupling pumps kinetic energy in a
   deterministic escape — worst-case ~18 s over random ICs, ~127 s for the
   deterministic cold-start IC, across RK4 (two dt), symplectic Verlet, and
   DOP853.
2. **Fixed-point saturation manufactured a phantom "attractor".** All Q32.32
   states saturate at ±2^31 (saturating i64). Long-horizon integration produced
   a bounded-looking trajectory whose converged spectrum (λ1 ≈ 1.46, KS ≈ 2.9
   nats/s at T=8000 s) was the exponent of the **saturated** system, not the
   ODE. There was no attractor in exact arithmetic to average over — KS entropy
   was not defined at the protocol epoch.
3. **A vectorized-batch bug nearly produced a false negative.** During the
   redesign verification, the batched Python `deriv` used row-index slices
   (`x[:N]`, `x[N:]`, `d[N:]`) on a `(K, 6)` array. The intended
   shape-agnostic derivative silently became *damping-only* past row 5. This
   manufactured a spurious "only 11-14% chaotic, mean λ ≈ −0.075" result that
   briefly had the bounded redesign looking dead on arrival.

## Root Cause Analysis

- **Design flaw (unbounded coupling):** the per-bob energy-window argument
  (kick energy 4.5 J < rotational barrier 19.6 J) was *local*. The coupling is
  a global well-avoiding parabola, so "bounded" was never implied once the
  chain was coupled. A textbook example of validating the wrong scope.
- **Numerics (saturation):** fixed-point determinism came at the price of a
  hard ceiling that looks like a bound. The mistake was treating "bounded
  output" as "bounded system". Q32.32 is exact and reproducible for the
  *transient*; it is a fidelity ceiling, not a confinement argument.
- **Verification footgun (batched deriv):** the bug was invisible to serial
  code, only present in the vectorized fast path — exactly the code path used
  to batch-sweep 40 ICs. Shape-agnostic helpers (operate on the last axis)
  passed bit-exact vs the scalar reference afterward.

## What Went Wrong (chronology)

1. Committed `T=100 s` data (robustness sweep, λ_min series, validate_benettin
   matches) that was internally consistent and cross-validated — but described
   the **bounded-swing transient** of an unbounded model.
2. Extrapolated the transient rate to a 1200 s epoch and quoted `dt_bound` as
   a security bound without ever integrating past the escape horizon.
3. First cross-check surfaced the escape (~100-200 s); verification then
   exposed the saturation mechanism, and the comparative epoch-cap analysis
   proved no tested variant reached 256 bits within its escape-free window.
4. The verification itself hit the batched-deriv bug, requiring a bit-exact
   scalar-vs-batched audit before trusting any sweep number.

## Resolution

- **Reframe first (ADR-003):** pendulum documented as a transient chaotic
  conditioner of physical noise; all committed data tagged as transient
  statistics; manuscript numbers gated behind `verify_metastability.py`
  (verify-before-trust).
- **Redesign (ADR-004):** replaced the linear coupling with the principal-value
  wrap `atan2(sin dtheta, cos dtheta)` — a sawtooth spring that turns over
  instead of ramping. Default coupling c=1.0 (CLI + legacy CLI + C-ABI
  keygen). Bounded by construction.
- **Re-validate:** float64 replicator (`validate_benettin.py`) vs Rust Q32.32 —
  all gated configs MATCH; the former L=0.5 "bifurcation cliff" now agrees
  exactly. `cargo test --release` 9 lib + 14 kat green, including three new
  wrapped-coupling tests (Jacobian slope-1 across branches, on-cut behavior,
  spin-boundedness).
- **Measure:** λ1 mean ≈ 0.405 / min ≈ 0.379 over 24–600 random ICs at
  T=2000–8000 s; KS ≈ 1.01–1.30 nats/s → **256-bit dt ≈ 136–176 s**,
  plausibly re-earning the original claim (final numbers still
  verifier-gated).

## Honest Assessment of the Recovery

The redesign works and is cross-validated, but the discipline that matters is
the order of operations that produced it: **verify the model bounds before
committing any rate**. The wrap is discontinuous at ±π (measure-zero — the ODE
is finite there and Benettin is unaffected), and it inherits honest weak bands
(c<0.35 near-critical, mass=0.5, length=4.0 at ~2200 s for 256 bits). The
fixed-point saturation floor and the finite-time/large-deviation caveats on any
restored 256-bit number remain open, and are explicitly listed as the
revisit-when triggers.

## Lessons Learned

| Lesson | Action that encodes it |
| :--- | :--- |
| A bounded-looking output is not a bounded system (saturation ceilings lie in both directions) | Fixed-point long-horizon claims require a bound on the discretization floor (ADR-003) |
| Validate the whole system's bounds, not just local ones | `verify_metastability.py` now checks c=0 control AND coupled escape across integrators |
| Vectorized fast paths can diverge from the scalar truth in silent ways | Shape-agnostic `deriv` + bit-exact scalar-vs-batched audit (before trusting sweep numbers) |
| Never extrapolate a finite window of an unbounded model | T=100 s data is permanently tagged transient (ADR-003 §2) |
| The float64 replicator is the referee, not the Rust core | All λ values are gate-checked against `validate_benettin.py` (ADR-001/002) |