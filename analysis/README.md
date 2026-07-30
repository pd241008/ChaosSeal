# analysis (Python)

Paper figures and statistics. Reads `/results/*.json` only — never recomputes protocol logic.

## Scripts

| Script | Purpose |
|--------|---------|
| `stats.py` | Prints actual numbers (mean / median / p95) quoted in the paper |
| `figures.py` | Generates the 3 paper figures via matplotlib |

## Figures

1. BEE ciphertext size vs |R|
2. Resynchronization latency distribution
3. Throughput comparison (ChaosSeal vs TLS 1.3 vs BPSec)

## Style

- Serif fonts
- No gridlines by default
- Vector PDF output

## Build & Test

```bash
cd analysis
pip install matplotlib numpy
make figures
```

## Status

Placeholder. Full implementation pending.
