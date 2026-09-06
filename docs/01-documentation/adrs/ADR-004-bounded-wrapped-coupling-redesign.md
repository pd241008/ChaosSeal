# ADR 004: Bounded (Wrapped) Elastic Coupling Replaces the Unbounded Linear Term

## Status
Accepted (2026-09-06)

## Context
ADR-003 demoted the pendulum to a transient conditioner because the linear
elastic coupling is globally unbounded (deterministic energy escape ~17–130 s;
fixed-point long-horizon "attractors" are ±2^31 saturation artifacts). The
design question reopened: can the ODE be made genuinely bounded *and*
robustly chaotic, restoring a cross-validated KS-entropy rate?

## Options Considered
1. **Wrapped linear coupling** — `wrap(d) = atan2(sin d, cos d)` in
   `(-pi, pi]`, coupling torque `c * wrap(dtheta)/d * 0.1`. The spring turns
   over at ±π instead of ramping torque forever. Thermally "correct" sawtooth
   spring, bounded by construction.
2. **tanh / sine couplings** — smooth bounded atomics (`c*tanh(d/s)`,
   `c*sin(d)`). Bounded, but measured min/robustness across random ICs came in
   weaker (chaotic fraction 0.875–0.975 vs 1.000 for wrap at c=1.0) and the
   smooth wells had narrower attractor margins.
3. **Polyramp / clipped-linear** — bounded but non-smooth-kink variants; adds a
   new parameter without improvement over wrap.

## Decision
Adopt **wrapped linear coupling** with default **c = 1.0**:

- `derivatives()` computes `(theta_i - theta_j).wrap()` everywhere the linear
  term was used; `Q32_32::wrap = self.sin().atan2(self.cos())` with an
  f64-fallback `atan2` (same convention as the existing `exp`/`ln` libm
  fallbacks), so the wrap reproduces the float64 reference to host precision.
- **Jacobian is unchanged structurally**: the wrap is piecewise linear with
  slope +1 almost everywhere, so the existing analytic Jacobian entries
  (`+/- c*0.1/d`, damping outside `/inertia`) still hold; only the branch cut
  at odd multiples of π is new (measure-zero, ODE value finite there — pinned
  by dedicated tests).
- Default coupling flips from 0.5 → 1.0 in the CLI, legacy CLI, and the C-ABI
  epoch-keygen path (the deployed entropy path).

Commits: `1cc6726` (bounded-coupling exploration + selection dataset),
implementation in `9eff403`.

## Reasoning
The wrap attacks the failure at its source: the coupling potential is confined
to a per-pair well, so there is no escape mechanism left. It also wins on the
measured robustness axis — at c=1.0 it was the only candidate with 100%
chaotic fraction over random ICs, lowest variance, and a positive minimum
across tested basins — while tanh/sin leave weaker attractor margins. The
f64-fallback atan2 keeps the cross-simulator validation tight (the whole
honesty chain depends on float64 replicator agreement).

## Consequences
- **Good** 🟢: Bounded (max|omega| ≈ 6.9 over 20000 s); robustly chaotic
  (λ1 mean ≈ 0.405, min ≈ 0.379 over 24–600 random ICs at T=2000–8000 s);
  float64 replicator vs Rust matches 8/8 gated configs — the former L=0.5
  bifurcation-cliff case now agrees exactly. KS ≈ 1.01–1.30 nats/s →
  256-bit dt ≈ 136–176 s, i.e. the bounded design plausibly re-earns the
  original claim (final numbers still verifier-gated per ADR-003).
- **Bad** 🔴: The wrap is discontinuous at the branch cut (torque jump
  ~2π·c·0.1/d); Benettin is unaffected (measure-zero, custom tests pin the
  on-cut ODE). Parameter sensitivity is now the honest limit — weak bands at
  c<0.35 (near-critical), mass=0.5, length=4.0 (256-bit ≈ 2200 s).
- **Neutral** 🟡: The epoch-keygen entropy path changed behavior (default
  coupling 1.0); netsim goodput framing is unaffected (per-bundle overhead).

## Revisit When
- A verifier's independent reproduction contradicts the measured rates, or
- The deployment wants an even larger entropy margin (e.g., sweep up N=4-5
  bobs), or c=1.0 is re-derived from a security-number criterion rather than
  the entropy-max heuristic used here.