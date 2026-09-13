# Reviewer Guide

This artifact accompanies the paper on dynamic node revocation and
chaos-derived key rotation in LEO satellite swarms (CEP: Broadcast Exclusion
Encryption + a chaos-derived key schedule). The repository contains multiple
independent evaluation components. Reviewers do not need to inspect the
repository linearly.

## Quick Navigation

| If you are reviewing... | Start here |
|---|---|
| Protocol architecture | `docs/architecture.md` |
| Formal security + membership arguments | `docs/end_to_end_security_and_membership.md` |
| Chaos/entropy design and its history | `docs/design_note_metastability.md` |
| Claim-to-evidence mapping | `CLAIM_MAP.md` |
| Review verification checklist | `REVIEW_CHECKLIST.md` |
| Fast verification (no full re-run) | `VERIFY.md` |
| Full reproduction | `REPRODUCE.md` |
| Reproduction guarantees by level | `REPRODUCIBILITY_LEVELS.md` |
| What to expect when running experiments | `EXPECTED_OUTPUTS.md` |
| Known limitations | `LIMITATIONS.md` |
| Methodological history (incl. retracted numbers) | `PROVENANCE.md` |
| Versioning and citation | `CITATION_TO_ARTIFACT.md` |
| Measured V3 results summary | `docs/v3_results.md` |

## Suggested Review Paths

### 15-minute scientific sanity check
1. `CLAIM_MAP.md` — understand the claim structure
2. `docs/v3_results.md` — the measured headline results
3. `docs/02-postmortems/2026-09-06-metastable-pendulum-saturation-artifact.md` — what was retracted and why
4. `LIMITATIONS.md` — boundaries of the artifact

### 30-minute methodology review
1. `docs/architecture.md` — Rust core / Go netsim / Python analysis contract
2. `docs/end_to_end_security_and_membership.md` — Part A (composition) and Part B (membership)
3. `docs/design_note_metastability.md` — bounded wrapped coupling, §7
4. `docs/01-documentation/adrs/` — ADR-001..004 (estimator correctness, coupling redesign)
5. `core_v2/README.md` and `netsim_v2/README.md` — component contracts

### 2-hour reproducibility review
1. `VERIFY.md` — build, tests, and verification gates
2. `REPRODUCE.md` — full reproduction instructions
3. Run selected `make reproduce-*` targets (all are minutes-scale)
4. `EXPECTED_OUTPUTS.md` — verify outputs match expectations

### 30-minute provenance review
1. `PROVENANCE.md` — evolution from legacy linear coupling to wrapped coupling
2. `docs/02-postmortems/` — the saturation-artifact postmortem
3. `docs/01-documentation/adrs/` — decision rationale
4. Compare `core/`/`netsim/` (legacy generation, retained for provenance) with
   `core_v2/`/`netsim_v2/` (canonical evaluation pipeline)

## Simulation Caveats

- All results are from the **v3 pipeline** (`core_v2` + `netsim_v2` +
  `analysis/` + `scripts/`). The legacy `core/`/`netsim/` tree is retained for
  provenance and is not used by any canonical claim.
- All security numbers derived from the pendulum are **design-stage and
  verifier-gated**; see the caveat block in `README.md` and
  `docs/design_note_metastability.md`.
- Reviewers with limited time can verify all claims using the archived
  `results_v3/` data plus the aggregation scripts, without re-running
  simulations.
