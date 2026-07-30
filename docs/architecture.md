# ChaosSeal Architecture

## System Overview

ChaosSeal is a reference implementation of a revocation and resynchronization protocol for LEO satellite constellations. The system is structured as a monorepo with four language-specific workspaces that communicate through well-defined interfaces.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        ChaosSeal Monorepo                       │
├──────────────┬──────────────┬──────────────┬───────────────────┤
│   /core      │   /netsim    │  /analysis   │   /dashboard      │
│   (Rust)     │   (Go)       │  (Python)    │   (Next.js)       │
│              │              │              │                   │
│ Protocol     │ Simulation   │ Paper        │ Live operator     │
│ engine,      │ driver,      │ figures      │ console,          │
│ KATs,        │ TLS 1.3 &    │ (matplotlib) │ replay,           │
│ C ABI        │ BPSec        │ stats.py     │ comparison view   │
│              │ baselines    │              │                   │
└──────┬───────┴──────┬───────┴──────┬──────┴─────────┬─────────┘
       │              │              │                │
       ▼              ▼              ▼                ▼
   libchaosseal    goroutines    /results/*.json   read-only
   _core.so/.a    + cgo/FFI      (single source    API route
                        │         of truth)           │
                        ▼                            │
                 real network                        │
                 events, timestamps,                 │
                 byte counts                         │
                                                ┌─────┴─────┐
                                                │   /results │
                                                │  *.json    │
                                                └───────────┘
```

## Data Flow Contract

1. **Rust → Go:** Go calls the Rust CLI (`chaosseal lyapunov ...`, `chaosseal beesize ...`) or links against `libchaosseal_core`. The Rust side emits JSON to stdout; Go parses it.
2. **Go → results:** Every simulation run writes exactly one JSON file to `/results`. The schema is documented in `netsim/README.md`.
3. **results → Python:** `analysis/` scripts read `/results/*.json` only. They never recompute protocol logic.
4. **results → dashboard:** The dashboard reads the same JSON files via a read-only API route or static serving. It must not reimplement protocol or simulation logic.

## Component Details

### core (Rust)

The source of truth for all protocol logic.

| Module | Purpose |
|--------|---------|
| `fixed` | Q32.32 fixed-point arithmetic (no floats in hot path) |
| `kinematics` | Multi-pendulum ODE system + RK4 integrator, fixed-point throughout |
| `lyapunov` | Benettin algorithm for Lyapunov exponent estimator (`λ1`) |
| `crypto` | HKDF → AES-256-CTR (vetted crates), HMAC-SHA256 commitment |
| `bee` | Subset-difference key tree, covering-set algorithm, ciphertext serialization |
| `bindings` | C ABI via `cbindgen` for cgo consumption |
| `bin` | CLI (`clap`) that emits JSON |

### netsim (Go)

Network simulation orchestration. Drives N satellite goroutines + 1 ground station goroutine, each calling into the Rust core.

| Feature | Implementation |
|---------|----------------|
| LEO link model | Visibility windows, elevation-dependent latency, Gilbert-Elliott loss bursts |
| TLS 1.3 baseline | Real `crypto/tls` handshake over simulated link |
| BPv7 / BPSec baseline | CBOR-encoded bundles (RFC 9171) with BIB (HMAC-SHA256) and BCB (AES-256-GCM) canonical blocks (RFC 9172) |
| Results emission | One JSON file per run to `/results` |

### analysis (Python)

Reads `/results/*.json` only. Never recomputes protocol logic.

| Script | Purpose |
|--------|---------|
| `stats.py` | Prints actual numbers (mean / median / p95) quoted in the paper |
| `figures.py` | Generates the 3 paper figures via matplotlib |

### dashboard (Next.js)

Live operator console. **Not the source of truth for paper numbers.**

| View | Description |
|------|-------------|
| Live swarm state | Real-time satellite positions and link status during a run |
| Revocation events | Stream of BEE revocation events as they happen |
| Replay | Step-through view for completed runs |
| Comparison | CEP vs TLS vs BPSec for quick eyeballing |

## Reproducibility

Every `/results/*.json` file records:

| Field | Description |
|-------|-------------|
| `git_commit` | Full commit hash at time of run |
| `rng_seed` | 64-bit seed used for the simulation |
| `parameters` | Complete parameter set (pendulum count, masses, damping, BEE `N`/`R`, loss model coefficients, etc.) |

To reproduce a specific run:

```bash
git checkout <commit_hash>
cd netsim
go run . --seed <rng_seed> --config <parameter_json>
```
