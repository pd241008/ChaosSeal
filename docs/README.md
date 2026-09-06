# ChaosSeal Docs

Index of documentation in this repo. ADR/postmortem content is mirrored from
the central Design-Dungeons repo (`pd241008/Design-Dungeons`); the mirrored
copies here are the project-local convenience copies.

## Advice records (ADRs)

| # | ADR | Topic |
|---|-----|-------|
| 001 | [`01-documentation/adrs/ADR-001-benettin-tangent-jacobian-flow.md`](01-documentation/adrs/ADR-001-benettin-tangent-jacobian-flow.md) | Correct Benettin tangent update (Jacobian flow, not constant matrix) |
| 002 | [`01-documentation/adrs/ADR-002-jacobian-inertia-placement.md`](01-documentation/adrs/ADR-002-jacobian-inertia-placement.md) | Jacobian inertia placement (damping outside /I, coupling inside /I) |
| 003 | [`01-documentation/adrs/ADR-003-entropy-claim-reframe-on-metastability.md`](01-documentation/adrs/ADR-003-entropy-claim-reframe-on-metastability.md) | 256-bit/epoch claim reframed onto metastability; verify-before-trust |
| 004 | [`01-documentation/adrs/ADR-004-bounded-wrapped-coupling-redesign.md`](01-documentation/adrs/ADR-004-bounded-wrapped-coupling-redesign.md) | Bounded (wrapped `atan2`) coupling redesign, default c=1.0 |

## Postmortems

- [`02-postmortems/2026-09-06-metastable-pendulum-saturation-artifact.md`](02-postmortems/2026-09-06-metastable-pendulum-saturation-artifact.md) — the metastable pendulum + fixed-point saturation "phantom attractor".

## Analysis / design notes

- [`design_note_metastability.md`](design_note_metastability.md) — the entropy-claim investigation, §7 resolution (wrapped coupling).
- [`architecture.md`](architecture.md), [`PROGRESS.md`](PROGRESS.md), [`v3_results.md`](v3_results.md) — architecture, progress/commit index, V3 results.
- [`end_to_end_security_and_membership.md`](end_to_end_security_and_membership.md) · [`manuscript_v3_patch.md`](manuscript_v3_patch.md) — manuscript-facing docs (design-stage numbers, verifier-gated).

## Verification gates

- `scripts/verify_metastability.py` — model-bounds proof (linear-coupling historical fact).
- `scripts/validate_benettin.py` — float64 replicator vs Rust Q32.32 (all gated configs MATCH).