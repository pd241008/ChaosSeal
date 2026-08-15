# ChaosSeal Progress

## Overview

This document tracks the implementation progress of the ChaosSeal protocol reference implementation. The project follows a deliverable-based approach with feature branching.

## Deliverables Status

| # | Deliverable | Status | Branch | Notes |
|---|-------------|--------|--------|-------|
| 1 | Rust core with KATs passing | ✅ Complete | `main` | Q32.32, RK4, Lyapunov, AES-CTR, HMAC-SHA256, BEE, C ABI, CLI |
| 2 | Go netsim + TLS 1.3 baseline + BPSec baseline | ✅ Complete | `feature/netsim` | LEO link model, 24-sat constellation, real TLS/BPSec baselines, JSON results |
| 3 | Python analysis producing 3 paper figures | ✅ Complete | `feature/analysis` | `stats.py`, `figures.py`, `Makefile`; verified against 15 real runs |

## Branching Strategy

- `main` — always deployable, reflects the paper-ready state
- `feature/<name>` — one branch per deliverable/component
- Work is done on `feature/*`, then PR'd into `main`

## Commit History (Modular)

| Commit | Component | Description |
|--------|-----------|-------------|
| `f2100d1` | root | Project scaffolding, .gitignore, top-level README |
| `7558753` | core | Q32.32 fixed-point arithmetic module |
| `7e933b1` | core | Multi-pendulum ODE system and RK4 integrator |
| `e194737` | core | Lyapunov exponent estimator (Benettin algorithm) |
| `08e5838` | core | AES-256-CTR and HMAC-SHA256 primitives |
| `3cffdca` | core | BEE key tree, covering set, and serialization |
| `6e9b4b5` | core | C ABI bindings and CLI entry point |
| `e0edbe3` | core | KATs for AES-CTR and HMAC-SHA256, cbindgen config |
| `ffe791a` | docs | Architecture doc, progress tracker, component READMEs |
| `e647bbf` | docs | Replace ASCII and PNG diagrams with Mermaid in README |
| `88c0792` | docs | Move progress tracker to `docs/` |
| `7e09ffe` | core | Remove dead code and unused imports (silences all build/test warnings) |
| — | netsim | LinkStats gains `latency_samples_ms` (per-visible-link one-way latency) |
| — | analysis | `stats.py`, `figures.py`, `Makefile`; 3 paper figures + stats over real runs |

## Next Steps

1. Create `feature/dashboard`:
   - Next.js app with live/replay/comparison views

## Known Issues

- Rust core compiles and tests pass with no warnings; BEE covering set size is a simplified heuristic, not a full subset-difference implementation.
- netsim baselines transmit at the strongest link moment (`bestMeasurement`), not at a fixed offset, because the 24-sat / 600 s window has intermittent visibility (`visible_pct` ≈ 0.7%).
- `visible_pct` ≈ 0.7% is suspiciously low for a real 24-satellite LEO constellation and should be investigated before the paper runs are finalized (a visibility bug would undercut link/latency realism).
