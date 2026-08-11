# netsim (Go)

Network simulation orchestration for LEO satellite constellations.

## Features

| Feature | Implementation |
|---------|----------------|
| LEO link model | Visibility windows, elevation-dependent latency, Gilbert-Elliott loss bursts |
| Satellite goroutines | N simulated satellites + 1 ground station |
| TLS 1.3 baseline | Real `crypto/tls` handshake over simulated link |
| BPv7 / BPSec baseline | CBOR-encoded bundles (RFC 9171) with BIB/BCB (RFC 9172) |
| Results emission | One JSON file per run to `/results` |

## Design Decisions

- **BPSec implementation:** Implemented directly against RFC 9171/9172 rather than depending on a Go library, because no viable Go ≥ 1.22-compatible dependency existed at time of writing.
- **Rust integration:** Go drives the Rust core via subprocess CLI (JSON on stdout) rather than cgo, to keep the Go build simple and avoid ABI fragility.

## Results Schema

Each run produces `/results/<run_id>.json` containing:

```json
{
  "run_id": "uuid",
  "git_commit": "full-sha",
  "rng_seed": 12345,
  "parameters": { ... },
  "events": [ ... ]
}
```

## Build & Test

```bash
cd netsim
go build ./...
go vet ./...
go test ./...
```

## Usage

```bash
# From the repo root (defaults use a root-relative Rust CLI path):
go run ./netsim --seed 12345 --results-dir results

# Or point the simulator at the release-built Rust core:
go run ./netsim --seed 12345 --rust-cli core/target/release/chaosseal

# Override simulation parameters:
go run ./netsim --satellites 48 --duration 600 --baselines chaosseal,tls13,bpsec
```

Run `go run ./netsim --help` for the full flag list. Every stochastic and
geometric input derives from `--seed`, so a run reproduces exactly given the
same flags and git commit.

## Status

Implemented and tested (`go test ./...` green). Smoke-tested end-to-end with
the release-built Rust core: a 24-satellite / 600 s constellation run produces
all three baselines with real cryptography (TLS 1.3 handshake, BPv7 bundle
with BIB+BCB, Rust Lyapunov + BEE revocation broadcast) and writes
`results/<run_id>.json`.
