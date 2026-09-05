#!/usr/bin/env python3
"""Regenerate the per-trial lambda_min distribution data under the corrected
Jacobian-based Benettin estimator.

Runs `cli_v2 lyapunov-attractor --samples 1000 --steps 10000` at the default
operating point (3 pendulums, m=1.0, L=1.0, b=0.1, c=0.5) for ten independent
1000-sample draws, mirrors the schema of the committed
results_v3/v4_lambda_min_series.csv, and overwrites it.

The column names are retained from the previous (pre-fix) dataset for backward
compatibility; with the corrected tangent update the spurious 60-75 nats/s
high band no longer exists, so highband_frac is ~0 and lowband_mean is the mean
of the (single, physical) band.

Run:  python3 scripts/regen_lambda_min_series.py
"""
import csv
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CLI = os.path.join(ROOT, "core_v2", "target", "release", "cli_v2")
OUT = os.path.join(ROOT, "results_v3", "v4_lambda_min_series.csv")
SAMPLES = 1000
STEPS = 10000
TRIALS = 10
HIGH_BAND_CUTOFF = 60.0  # nats/s; retained for schema compatibility

BASE = ["--pendulums", "3", "--mass", "1.0", "--length", "1.0",
        "--damping", "0.1", "--coupling", "0.5",
        "--samples", str(SAMPLES), "--steps", str(STEPS)]


def run_trial():
    res = subprocess.run([CLI, "lyapunov-attractor"] + BASE,
                         capture_output=True, text=True, timeout=600)
    if res.returncode != 0:
        raise RuntimeError(res.stderr)
    return json.loads(res.stdout)["output"]


rows = []
for trial in range(1, TRIALS + 1):
    d = run_trial()
    raw = d["raw_lambda1"]
    lows = [x for x in raw if x < HIGH_BAND_CUTOFF]
    hi_frac = 1.0 - len(lows) / len(raw)
    rows.append({
        "trial": trial,
        "lambda_min_nats_per_s": f"{min(raw):.12f}",
        "lambda_max_nats_per_s": f"{max(raw):.12f}",
        "highband_frac": f"{hi_frac:.4f}",
        "lowband_mean_nats_per_s": f"{(sum(lows) / len(lows) if lows else 0.0):.12f}",
    })
    print(f"trial {trial}: min={rows[-1]['lambda_min_nats_per_s'][:9]} "
          f"max={rows[-1]['lambda_max_nats_per_s'][:9]} "
          f"highband={hi_frac:.3f} mean(low)={rows[-1]['lowband_mean_nats_per_s'][:9]}")

with open(OUT, "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
    w.writeheader()
    w.writerows(rows)

mins = [float(r["lambda_min_nats_per_s"]) for r in rows]
lo_means = [float(r["lowband_mean_nats_per_s"]) for r in rows]
print(f"\nWrote {len(rows)} trials to {OUT}")
print(f"lambda_min: mean={sum(mins)/len(mins):.4f} min={min(mins):.4f} "
      f"max={max(mins):.4f}")
print(f"max dt_bound (256ln2/lambda_min) over trials: "
      f"{max(256*0.6931471805599453/m for m in mins):.1f} s")