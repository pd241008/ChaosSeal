# dashboard (Next.js)

Live operator console for the ChaosSeal simulation.

**This is NOT the source of truth for paper numbers.** It reads the same `/results/*.json` files as the Python analysis pipeline.

## Views

| View | Description |
|------|-------------|
| Live swarm state | Real-time satellite positions and link status during a run |
| Revocation events | Stream of BEE revocation events as they happen |
| Replay | Step-through view for completed runs |
| Comparison | CEP vs TLS vs BPSec for quick eyeballing |

## Constraints

- Must not reimplement any protocol or simulation logic
- Reads `/results/*.json` via a read-only API route or static file serving
- Dev/ops tool only

## Build & Test

```bash
cd dashboard
npm install
npm run dev
```

## Status

Placeholder. Full implementation pending.
