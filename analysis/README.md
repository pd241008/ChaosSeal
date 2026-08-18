# analysis (Python)

Paper figures and statistics. Reads `/results/*.json` only — never recomputes protocol logic.

## Scripts

| Script | Purpose |
|--------|---------|
| `stats.py` | Prints actual numbers (mean / median / p95) quoted in the paper |
| `figures.py` | Generates the 3 paper figures via matplotlib |

`stats.py` exposes the shared loading and metric helpers that `figures.py`
imports, so both scripts read exactly the same JSON fields.

## Figures

1. `figures/bee_size_vs_r.pdf` — BEE ciphertext size vs `|R|` (log-log, from the `bee-r` sweep runs)
2. `figures/resync_latency.pdf` — Distribution of one-way link latency over the visible pass (from `link_stats.latency_samples_ms`)
3. `figures/throughput.pdf` — Effective Goodput comparison (ChaosSeal vs TLS 1.3 vs BPSec vs Hybrid)

## Metric definitions (as computed in `stats.py`)

- **Revocation link latency** — `latency_ms` of each BEE revocation update (transmission happens at the strongest-link moment, so this is the closest-approach propagation delay).
- **Visible link latency** — every sampled one-way latency where the satellite was above the minimum elevation.
- **Goodput (Mbps)** — effective application payload bits per second of each baseline operation:
  - ChaosSeal/Hybrid: `payload_bytes*8 / (transfer_sec + crypto_wallclock)`
  - TLS 1.3: `1024*8 / (handshake_sec + app_payload_sec)`
  - BPSec: `payload_bytes*8 / transfer_sec` (transfer already includes propagation)

## Style

- Serif fonts
- No gridlines by default
- Vector PDF output

## Build & Test

```bash
cd analysis
pip install matplotlib numpy
make figures
make stats
```

## Status

Implemented. Verified against 15 real `netsim` runs (5 seeds at the paper's
parameter set plus a `|R|` sweep 1–512). All numbers in the paper must trace
to `stats.py` output over `/results/*.json`.
