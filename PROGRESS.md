# ChaosSeal Progress

## Overview

This document tracks the implementation progress of the ChaosSeal protocol reference implementation. The project follows a deliverable-based approach with feature branching.

## Deliverables Status

| # | Deliverable | Status | Branch | Notes |
|---|-------------|--------|--------|-------|
| 1 | Rust core with KATs passing | ✅ Complete | `main` | Q32.32, RK4, Lyapunov, AES-CTR, HMAC-SHA256, BEE, C ABI, CLI |
| 2 | Go netsim + TLS 1.3 baseline + BPSec baseline | 🔄 In Progress | `feature/netsim` | LEO link model, satellite goroutines, real TLS/BPSec baselines |
| 3 | Python analysis producing 3 paper figures | ⏳ Pending | — | matplotlib, publication style, stats.py |
| 4 | Next.js dashboard | ⏳ Pending | — | Live swarm state, replay, comparison view |

## Branching Strategy

- `main` — always deployable, reflects the paper-ready state
- `feature/<name>` — one branch per deliverable/component
- Work is done on `feature/*`, then PR'd into `main`

## Commit History (Modular)

| Commit | Component | Description |
|--------|-----------|-------------|
| `f2100d1` | root | Project scaffolding, .gitignore, top-level README |
| `84a303a` | core | Q32.32 fixed-point arithmetic module |
| `d56c073` | core | Multi-pendulum ODE system and RK4 integrator |
| `ebe97de` | core | Lyapunov exponent estimator (Benettin algorithm) |
| `31b0566` | core | AES-256-CTR and HMAC-SHA256 primitives |
| `b25edc0` | core | BEE key tree, covering set, and serialization |
| `da14838` | core | C ABI bindings and CLI entry point |
| `487488e` | core | KATs for AES-CTR and HMAC-SHA256, cbindgen config |
| `0e50af4` | root | Fix Mermaid diagrams in README |
| `0720bd9` | feature/netsim | Simplify Mermaid diagrams for GitHub rendering |

## Next Steps

1. Complete `feature/netsim`:
   - LEO link model with Gilbert-Elliott loss
   - Satellite goroutines + ground station
   - TLS 1.3 baseline (real handshake)
   - BPv7/BPSec baseline (RFC 9171/9172)
   - JSON results emission to `/results`

2. Create `feature/analysis`:
   - `stats.py` for mean/median/p95
   - `figures.py` for 3 paper figures (PDF output)

3. Create `feature/dashboard`:
   - Next.js app with live/replay/comparison views

## Known Issues

- GitHub Mermaid rendering requires `<br>` instead of `\n` for line breaks in node text. We've switched to PNG diagrams to avoid this issue.
- Rust core compiles and tests pass, but BEE covering set size is a simplified heuristic, not a full subset-difference implementation.
