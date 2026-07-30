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
go mod init github.com/chaosseal/netsim  # if first run
go test ./...
go run .
```

## Status

Placeholder. Full implementation pending.
