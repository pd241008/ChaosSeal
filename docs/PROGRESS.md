# ChaosSeal Progress

## Overview

This document tracks the implementation progress of the ChaosSeal protocol reference implementation. The project follows a deliverable-based approach with feature branching.

## Deliverables Status

| # | Deliverable | Status | Branch | Notes |
|---|-------------|--------|--------|-------|
| 1 | Rust core with KATs passing | ✅ Complete | `main` | Q32.32, RK4, Lyapunov, AES-CTR, HMAC-SHA256, BEE, C ABI, CLI |
| 2 | Go netsim + TLS 1.3 baseline + BPSec baseline | ✅ Complete | `feature/netsim` | LEO link model, 24-sat constellation, real TLS/BPSec baselines, JSON results |

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

## Next Steps

1. Create `feature/analysis`:
   - `stats.py` for mean/median/p95 over repeated seeded runs
   - `figures.py` for 3 paper figures (PDF output)

2. Create `feature/dashboard`:
   - Next.js app with live/replay/comparison views

## Known Issues

- Rust core compiles and tests pass, but BEE covering set size is a simplified heuristic, not a full subset-difference implementation.
- `cargo build --release` in `core/` emits one warning (unnecessary parentheses in `core/src/fixed/q32_32.rs:10`); harmless but could be cleaned up.
- netsim baselines transmit at the strongest link moment (`bestMeasurement`), not at a fixed offset, because the 24-sat / 600 s window has intermittent visibility (`visible_pct` ≈ 0.7%).
