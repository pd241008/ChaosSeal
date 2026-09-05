# Design note: metastability of the reinjected 3-pendulum ODE

Status: DRAFT. The quantitative claims below are reproduced by
`scripts/verify_metastability.py` (independent verification), not by any single
simulator. Per the verify-before-trust gate, no manuscript-figure or final
security-number changes are committed until the verifier confirms them.

## TL;DR

The reinjected 3-pendulum ODE with **linear elastic coupling**
`tau_c = c * ((theta_i - theta_{i-1}) / d) * 0.1` is **not a bounded chaotic
attractor** in exact arithmetic. Its coupling is a globally unbounded parabolic
potential (unconstrained theta), so after a few kick episodes the mean angular
velocity of pendulum 0 drifts relative to its neighbours, `Delta_theta` grows
without bound, and the coupling torque pumps kinetic energy in a deterministic
energy-escape.

Consequences:

* Float64 integration of the model escapes the swing basin at roughly
  100-200 s (max |omega| grows ~e^(0.18/s) thereafter, unreliably).
* fixed-point (Q32.32) integration is artifact-bounded: all state variables
  saturate at +/-2^31 (approx +/-2.15e9) via saturating i64 arithmetic, which
  manufactures a bounded-looking long-horizon "attractor" whose converged
  spectrum (lambda1 ~ 1.46, KS ~ 2.9 nats/s at T = 8000 s) is not the exponent
  of the system.
* The **committed T = 100 s-window data** (robustness sweep, lambda_min series,
  validate_benettin matches) are internally consistent and cross-validated, but
  they describe the **bounded-swing transient** of an unbounded model, valid
  only up to the escape horizon. They cannot be extrapolated to a 1200 s epoch,
  and the 256-bit special accumulation claim is not supported by any tested
  model variant at any measured minimum rate.
* Kolmogorov-Sinai entropy (`h_KS = sum lambda_k>0 lambda_k`, manuscript Eq. 10)
  is therefore not defined for the committed model at the protocol epoch: there
  is no attractor on which to average the exponents.

## 1. The finding

### 1.1 The model

State `x = [theta_0..2, omega_0..2]`; per pendulum i:

```
omega_i' = -b * omega_i
         + ( g * (m*2) * (L/2) * sin(theta_i)
           + c * ((theta_i - theta_{i-1}) / d) * 0.1 ) / inertia
theta_i' = omega_i
```

Coupling is to the left neighbour only (the self term j == i is identically
zero). At defaults m=L=1, b=0.1, c=0.5: gravity slope g = 9.8067 (rad/s2, per
unit angle), coupling slope 0.05 rad of torque per radian of `Delta_theta`.
The kick `omega_0 += 3.0` fires whenever `sum |omega| < 0.5` (deterministic
reinjection).

### 1.2 Single-pendulum energy bound does not apply to the chain

A single kicked pendulum is bounded: kick energy 0.5*m*(3.0)^2 = 4.5 J is far
below the rotational barrier 2*m*g*L = 19.6 J, so swings stay in the basin
forever (verified: c=0 control stays at max|omega| ~ 6.7-6.9 over 1200 s, 364
kicks).

In the chain the coupling potential `(c/d*0.1) * (theta_i-theta_j)^2 / 2` is a
global parabola. Pendulum 0 receives every kick, so its mean angular velocity
drifts; `theta_0 - theta_1` then grows without bound and the coupling torque
`0.05 * Delta_theta` grows with it, pumping kinetic energy. The energy barrier
is bypassed by the elastic mode, not the gravity mode.

### 1.3 Escape evidence (integrator-independent)

Default deterministic IC, default params, reinjection active. max|omega| per
100 s bin:

| horizon | RK4 dt=1e-2 | RK4 dt=1e-4 | Verlet dt=2e-3 | DOP853 rtol=1e-10 |
|---------|-------------|-------------|----------------|--------------------|
| 100 s   | 6.7e0       | 6.7e0       | 5.8e1*         | 6.7e0              |
| 200 s   | 1.9e8       | 1.4e8       | 1.2e9*         | 1.9e8              |
| 400 s   | 2.6e24      | 2.0e24      | 1.4e22*        | 2.6e24             |

(*Verlet starts transiently higher; the asymptotic signature -- escape after
100-200 s, then sustained ~e^(0.18/s) growth -- matches all integrators.)

The escape is step-size independent (1e-2 to 1e-4) and algorithm-independent
(RK4, symplectic Verlet, adaptive DOP853 with rtol=1e-10), hence it is a
property of the ODE, not of a particular solver.

## 2. Simulator artifacts at long horizons

### 2.1 float64

Once the trajectory enters the spin regime (|omega| > ~50), float64 RK4
amplifies launched spins (omega=50 -> 1e10 in 100 s; omega=1000 -> 1e11) that a
damped rotor must decay. Long-horizon float64 spectra (e.g. lambda ~ 0.22
nats/s at 2000 s) are therefore contaminated and not reliable.

### 2.2 fixed-point (Q32.32)

`core_v2/src/fixed/q32_32.rs` is i64 with 32 fractional bits and saturating
add/sub/mul (clamped to i64 bounds). Every state component saturates at
+/-2^31 ~ 2.15e9. Long-horizon fixed-point runs therefore pin to the cap and
produce a bounded-looking "converged" spectrum. Measured plateau at the
deterministic IC: lambda ~ (1.461, 1.460, 0.008) at T = 8000 s, KS ~ 2.93 --
this is an artifact, not the system exponent.

### 2.3 Mechanism nuance

Naive float64 clamping of the state to +/-2.15e9 (simulating the cap) does NOT
reproduce lambda ~ 1.46; it yields lambda ~ 0.005 (pinned, near-regular).
The artifact therefore arises from the fixed-point internals (quantization of
every operation, saturation inside products and the reinjection test, Q32.32
sin/cos approximant range-reduction at huge arguments), not from a plain state
cap. Conclusion is unaffected: the long-horizon fixed-point value is not the
physical exponent.

### 2.4 Bounding alternative explored and rejected

Replacing the linear coupling with a bounded sine coupling
`tau_c = c * sin(theta_i - theta_{i-1}) * 0.1` keeps the orbit bounded
(max|omega| stays ~6.8) but removes most entropy: at c = 0.5, lambda1 -> 0.002
nats/s (near-integrable); at c = 2.0 the attractor is bimodal -- 62% of random
ICs land in a chaotic basin (lambda1 ~ 0.33-0.39) but 38% land in a
near-integrable basin (lambda1 ~ 0.03). An IC-dependent entropy rate is not a
crypto-viable source. Bounding the model is therefore not pursued: the project
relies on chaos, and removing the unbounded coupling removes the chaos.

## 3. Status of committed data

All committed Lyapunov data were computed with T = 100 s windows (steps =
10000) on the linear-coupling model and cross-validated against float64 at the
same window (6/6 gated configs MATCH in `scripts/validate_benettin.py`).

| artefact | status |
|----------|--------|
| `results_v3/pendulum_robustness_sweep.csv` | cell values remain valid as bounded-swing-transient-window estimates; only inside the deterministic-IC envelope (~127 s); no attractor meaning |
| `results_v3/v4_lambda_min_series.csv` (mean 0.0256, range [0.0068, 0.0426] nats/s) | bounded-transient-window lambda_min distribution; NOT a stationary-attractor lambda_min; random-IC sampling exceeds the ~18 s worst-case escape envelope |
| [4165, 26095] s dt_bound and 6932 s nominal from the manuscript Section 6.4 prose | **invalid extrapolation** (assumes stationary attractor); escape kills the model past ~200 s |
| 256-bit accumulation in a 1200 s epoch claim | not supported by any tested variant at any measured guaranteed minimum rate |
| L=0.5 boundary divergence (f64 0.52 vs fixed 1.9, doc'd in validate script) | unrelated aberrancy (bifurcation cliff), reported separately |

## 4. Rewritten security framing: bounded-swing transient

The defensible claim is:

> The reinjected 3-pendulum ODE provides an entropy source via its
> bounded-swing transient dynamics. The instantaneous expansion rate is
> lambda1(t) ~ O(0.1-0.4 nats/s) within the bounded transient window. The
> window size is IC-dependent and LIMITED BY THE ESCAPE TIME (see below). All
> committed sweeps are estimates on that window and are only valid up to the
> escape horizon.

The role is reduced to a **seed-strengthener / VRNG-input conditioner**, not a
direct 256-bit-per-epoch extractor. Numerical claims must not use horizons
beyond the measured escape time.

Measured escape envelope (criterion max|omega| > 50; dt = 2e-3, batched over
24 random ICs uniform in [-pi,pi], seed 11):

| quantity | value |
|----------|-------|
| worst-case (min) escape time | 17.6 s |
| p05 | 17.8 s |
| median | 67.6 s |
| fraction escaped by 400 s | 100 % |
| deterministic cold-start IC (0.1,0.2,0.3,0,0,0) escape | ~127-129 s (RK4 at dt=1e-2/1e-3/1e-4), ~99 s (Verlet) |

Consequences for the committed data:

* The committed T = 100 s windows (robustness sweep, lambda_min series) are
  OUTSIDE the worst-case escape envelope (17.6 s) when sampled over random ICs.
  A strict claim requires either (a) window/epoch <= ~15 s and recomputed data,
  or (b) a protocol restricted to the deterministic cold-start IC, whose 100 s
  window is inside its ~127 s envelope.
* Short windows (15-20 s) give noisy, weakly-converged exponent estimates
  (~200 Gram-Schmidt renormalizations at steps=2000), so a worst-case-min-rate
  number at the strict cap is of marginal statistical quality.

## 5. Epoch restriction

Protocol epochs, extraction windows, and any dt_bound calculation MUST be
capped at (or below) the worst-case (minimum) escape time of the deployed
parameter set. With random-IC resampling per epoch the cap is ~17 s (measured);
with a deterministic cold-start IC the cap is ~127 s (measured). The committed
100 s-window data is inside the latter envelope only.

The single-pendulum bound (kick energy 4.5 J < barrier 19.6 J) is NOT a global
bound once coupling is present (coupling is a global parabola avoiding the
barrier), so there is no ambient guarantee beyond these measurements.

### 5.1 Comparative analysis of the two caps (random-IC 15 s vs deterministic 100 s)

Ran `scripts/compare_epoch_policies.py` (27 configs; raw tables under
`results_v3/compare/`). The 15 s random-IC policy:

* lambda_min at window=15 s (steps=1500) is **negative across every config**
  (default point min ~ -0.19 to -0.07, mean-of-trial ~ -0.05 to -0.19): the
  window is far too short for the Benettin estimator to resolve positive
  expansion, so `dt_bound = 256 ln2 / lambda_min` is undefined.
* lambda1 (deterministic IC) is better-behaved but the valid window is
  config-limited: default 0.178 nats/s with escape at ~129 s gives ~26 bits per
  100 s window and `dt_bound_256 ~ 998 s >> 129 s` (only ~26 bits obtainable
  before escape). Several configs escape within 100 s (length 0.5 -> 6.1 s,
  damping 0.2 -> 99.1 s, damping 0.4 -> 94.6 s, coupling 0.8 -> 98.4 s);
  length 0.5 has the largest lambda1 (1.637) but only ~14 usable bits before its
  6.1 s escape.

Verdict of the comparison:

* (a) random-IC at <=15 s is not a statistically usable entropy regime
  (lambda_min < 0).
* (b) deterministic-IC at 100 s is inside the envelope for the default point but
  not for all configs, and yields only ~26 bits per bounded window (~10 windows
  short of 256), i.e. 256 bits cannot be accumulated within a single escape-free
  run.
* The deterministic-IC protocol is, per boot, deterministic: the model
  contributes **zero fresh entropy from sampling**; a model-based entropy claim
  is not supportable. The bounded swing transient only acts as a *conditioner*
  of physical noise that the hardware must supply.

Neither policy restores the original claim; the pendulum element is best
documented as a transient chaotic conditioner, not a 256-bit/epoch source.

## 6. Verified artefacts (for the verifier)

- Deterministic escape + C=0 bounded control (integrator-independent).
- Fixed-point saturation bound (+/-2^31) from `core_v2/src/fixed/q32_32.rs`.
- Clamp-f64 non-reproduction (mechanism nuance).
- Escape-time CDF over random ICs (epoch cap input).