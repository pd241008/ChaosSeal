# Verification

This guide covers **fast verification** of the artifact without re-running
the full evaluation sweeps. Verification confirms that the provided files and
scripts support the paper's claims. Reproduction (`REPRODUCE.md`) covers
end-to-end regeneration.

## Prerequisites

```bash
rustc --version        # >= 1.75
go version             # >= 1.22
python3 --version      # >= 3.10 (numpy + matplotlib for aggregation/figures)
```

## Step 1: Build and Unit-Test the Protocol Core

```bash
cd core_v2 && cargo build --release && cd ..
cd core_v2 && cargo test --release && cd ..
```

Expected output: build clean with no warnings; test summary
**9 lib + 14 KAT tests passed** (AES-CTR RFC 3686, HMAC-SHA256 RFC 4231,
determinism, wrapped-coupling tests).

## Step 2: Build and Test the Network Simulator

```bash
cd netsim_v2 && CGO_ENABLED=1 go build -o chaoseal-sim . && cd ..
cd netsim_v2 && go vet ./... && go test ./... && cd ..
```

Expected output: `go vet` clean; `go test ./...` green across
`core/client`, `core/crypto`, `core/engine`, `core/kinematics`, and the
`test/` integration suite.

## Step 3: Run the Independent Verification Gates

The float64 replicator is the referee, not the Rust core (ADR-001/002):

```bash
python3 scripts/validate_benettin.py
```

Expected output: `ALL MATCH` — float64 independent Benettin vs Rust Q32.32
agree on every gated configuration (mixed absolute+relative criterion for
near-zero exponents).

```bash
python3 scripts/verify_metastability.py
```

Expected output: linear-coupling model-bounds proof (c=0 control vs coupled
escape) across integrators. Optional heavier run:
`python3 scripts/verify_metastability.py --fine --dop853`.

## Step 4: One-End-to-End Simulation Run

```bash
RUSTCLI="$(pwd)/core_v2/target/release/chaosseal"
./netsim_v2/chaoseal-sim --run-id verify-smoke --seed 1 --bee-r 8 \
  --baselines chaosseal,counter,bpsec \
  --results-dir results_fresh --rust-cli "$RUSTCLI"
```

Expected runtime: **<1 s**.

Expected output: stdout lines `run_id=... satellites=24`,
`link: visible=... mean_latency=... loss=...`, one `baseline <name>: ok` per
baseline, and `results -> <path>`; exactly one JSON file written to
`results_fresh/` with `git_commit`, `rng_seed`, and full `parameters` embedded.

## Step 5: Regenerate the Aggregated Statistics From the Archive

```bash
python3 analysis/v3_analysis.py
```

Expected output: rebuilds `results_v3/v3_sweep_stats.csv`,
`v3_loss_sweep_stats.csv`, `v3_size_sweep_stats.csv`,
`v3_commit_sweep_stats.csv`, `v3_corruption_stats.csv`, and the figures under
`results_v3/figures/`. Values must match the numbers quoted in
`docs/v3_results.md`.

## Step 6: Spot-Check Claim Evidence Against the Archive

```bash
# C1: pendulum vs counter vs BPSec at one operating point
python3 -c "import json;d=json.load(open('results_v3/v3-rsweep-seed1-r0008.json'));print(json.dumps({k:d[k] for k in list(d)[:3]}, indent=2)[:400])"

# C2: corruption detection summary
python3 -c "import json;print(json.load(open('results_v3/v3-corruption-test.json'))['summary'])"

# C6: bootstrap CI summary
python3 -c "import csv;print(*csv.reader(open('results_v3/v3_crossover_bootstrap_stats.csv')))"

# C4: finite-horizon convergence table
column -s, -t results_v3/convergence/convergence_summary.csv | head
```

## What Not to Expect

- **Bit-identical wall-clock timings.** The Go simulator records real network
  events and wall-clock crypto timing; run-to-run timing jitter is expected
  (documented in `docs/v3_results.md` Tier 2a). All *derived* goodput
  quantities reproduce within noise; all *deterministic* core quantities
  (Q32.32 kinematics, Lyapunov estimates at fixed ICs, KATs) are exact.
- **A fast `verify_spectrum_convergence.py`.** The T=4000 s spectrum horizon
  runs 100 samples × 400k steps through the Q32.32 core (minutes, single
  core). The shipped CSVs under `results_v3/convergence/` are the archived
  output; re-run only if you need to.
- **Reproducing the archived JSONs by re-running the sim.** Each archived run
  embeds its `git_commit` and seed; replays require the same flags and
  commit. Prefer comparing freshly generated statistics against
  `docs/v3_results.md` rather than diffing JSON files.
