# ADR 002: Jacobian Inertia Placement — Damping Outside, Coupling Inside the /I Division

## Status
Accepted (2026-08-20)

## Context
The pendulum ODE computes, per bob `i`:

```
omega_i' = -b_i * omega_i            // damping term is OUTSIDE the /inertia division
         + (torque_g + torque_c) / inertia_i
```

Damping is not divided by inertia in `derivatives()` (a deliberate literal
reading of the model), while gravity and coupling torques are. The analytic
Jacobian must reproduce exactly that split: the diagonal entry for omega_i
w.r.t. omega_i must be `-b_i` (NOT `-b_i / I`), and the coupling entries for
omega_i w.r.t. theta_{i-1} / theta_i must carry `/ I_i`.

At the default `m=1.0, L=1.0` config, inertia = 1, so a `-b/I` slip is a
no-op and invisible to every test that only runs defaults. Only configs with
non-unit inertia expose the bug.

## Decision
Place the Jacobian entries to mirror the literal derivative split of
`derivatives()`:

- `J[omega_i][omega_i] = -b_i` (no inertia factor).
- `J[omega_i][theta_{i-1}] = -c*0.1/d / I_i`; `J[omega_i][theta_i] =
  (g*(2m)*(L/2)*cos(theta_i) + c*0.1/d) / I_i`.

Then regenerate the committed robustness sweep from the fixed binary.
Commit: `04ed795` (`fix(lyapunov): correct jacobian inertia placement;
regenerate robustness sweep`).

## Reasoning
A Jacobian that is faithful in the default config but wrong under parameter
variation quietly corrupts two things at once: the Lyapunov spectrum (tangent
flow) and the parameter-robustness sweep. Inertia = 1 configs give false
confidence. Non-unit-inertia configs in `kat.rs` and `validate_benettin.py`
(0.25, 0.5, 1.125, 4, 6, 16) are the only way to catch it, so the verifier
suite deliberately includes them.

## Consequences
- **Good** 🟢: finite-difference and tangent-product identities hold at every
  probed inertia; the sweep regenerates from the corrected binary.
- **Bad** 🔴: any λ / robustness numbers produced by the pre-fix build in the
  non-default-config region were re-measured (again feeding the reframe ADR-003).

## Revisit When
- The damping term is reframed as a bulk viscous torque (i.e., `-b*omega`
  moves inside `/inertia`). That is a model change, not a bug fix, and requires
  new validation.