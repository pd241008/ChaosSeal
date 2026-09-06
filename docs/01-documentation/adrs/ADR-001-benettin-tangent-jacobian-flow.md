# ADR 001: Benettin Tangent Update Must Follow the Linearized Jacobian Flow

## Status
Accepted (2026-08-20)

## Context
The Lyapunov estimator in the Rust core (`core_v2/src/lyapunov`) ran a Benettin
scheme in which the tangent (perturbation) vectors were integrated with an
update that did not match the flow linearization of the pendulum ODE. The
estimator we cross-validated against, and the `.kiev`-style reference used for
the manuscript, integrated tangents with `v <- v + J(x) v dt` where `J(x)` is
the exact analytic Jacobian of `derivatives()`. A mismatch between the tangent
update and the true linearized flow manufactures Lyapunov exponents that are
numerically stable but not exponents of the system — and a stability check
(`dt_bound = 256 ln2 / lambda`) built on top of such exponents is unfalsifiable.

Also present: the first-order tangent Euler update is only consistent with the
RK4-trajectory integration if the Jacobian is sampled at the **advanced** state
each step. Using stale/initial-only Jacobians biases the estimates.

## Decision
Correct the Benettin tangent update to use the linearized Jacobian flow
exactly:

- Integrate each tangent vector with `v <- v + J(x_after_step) v dt` at every
  integration step, using the full 6x6 analytic Jacobian of the pendulum.
- Re-orthonormalize every 10 steps with a Gram-Schmidt pass over the tangent
  basis, exactly as the float64 reference implementation does.
- Treat "exponent" as `log(growth_per_interval) / dt / N_intervals`, matching
  the reference's accumulation convention.

Commit: `f68bdc5` (`fix(lyapunov): correct Benettin tangent update to the
linearized Jacobian flow`).

## Reasoning
A Lyapunov spectrum is only meaningful if the tangent dynamics are the
linearization of the simulated ODE along the simulated trajectory. Any other
tangent update produces "exponents" of some other (unstated) dynamical system.
Since the whole entropy argument reduces to a measured rate, we made the
estimator bit-consistent with an independent float64 replicator
(`scripts/validate_benettin.py`) rather than trusting either implementation.
The replicator is the cross-check; the Rust code is the production path.

## Consequences
- **Good** 🟢: `validate_benettin.py` agrees to 6/6 across non-unit-inertia
  configs; the Jacobian and the ODE now commute under finite-difference and
  tangent-product identity tests.
- **Bad** 🔴: The correction changed λ values versus the pre-fix build —
  earlier numbers that were produced with the wrong tangent update became
  suspect and had to be re-derived (this fed directly into the
  claim-reframe ADR-003).

## Revisit When
- A different estimator formulation (e.g., QR-decomposition spectrum rather
  than Gram-Schmidt) is adopted, or the integration scheme (RK4 → symplectic)
  changes, prompting a re-derivation of the tangent update.