#!/usr/bin/env python3
"""Regenerate pendulum-robustness sweep data from the actual CLI binary.

Runs lyapunov-attractor at each parameter point, extracts the low-band λ_min
(physical minimum only, excluding the old ~15% blow-up artifact band -- under
the wrapped (bounded) coupling design, the blow-up band is ~0), and writes a
committed CSV so v4_generalization.pendulum_robustness() is reproducible.

NOTE (bounded redesign): the sweep now exercises the wrapped-atan2 coupling
(commit after 1cc6726) and the DEFAULT coupling moved to 1.0 (the entropy-max
design; see scripts/explore_bounded_coupling.py). All held-at-default points
use c=1.0. The horizon moved from T=100 s to T=2000 s: at T=100 s the
Benettin slot-1 exponent has not yet separated from slot-2/slot-3, producing
near-zero/negative "minima" that are pure convergence-lag artifacts -- the
converged (2000 s) minima are strictly positive everywhere (weakest point
L=4.0: 0.081). Sample count reduced from 500 to 150 to keep the longer
horizon affordable; means/minima agree with the 24-IC probes to ~3 digits.
"""
import subprocess, json, csv, sys, os
from concurrent.futures import ThreadPoolExecutor

CLI = os.path.join(os.path.dirname(__file__), "..", "core_v2", "target", "release", "cli_v2")
SAMPLES = 150
STEPS = 200000
OUT = os.path.join(os.path.dirname(__file__), "..", "results_v3", "pendulum_robustness_sweep.csv")

PARAMS = {
    "damping":  {"flag": "--damping",  "vals": [0.055, 0.06, 0.07, 0.08, 0.09, 0.10, 0.2, 0.4]},
    "coupling": {"flag": "--coupling", "vals": [0.2, 0.3, 0.4, 0.5, 0.6, 0.8, 1.0]},
    "length":   {"flag": "--length",   "vals": [0.5, 1.0, 2.0, 4.0]},
    "mass":     {"flag": "--mass",     "vals": [0.5, 1.0, 1.5, 2.0]},
}

def run_one(param_name, val, defaults):
    flags = []
    for p, v in defaults.items():
        if p == param_name:
            flags += [PARAMS[p]["flag"], str(val)]
        else:
            flags += [PARAMS[p]["flag"], str(v)]
    flags += ["--samples", str(SAMPLES), "--steps", str(STEPS)]
    cmd = [CLI, "lyapunov-attractor"] + flags
    try:
        res = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
        data = json.loads(res.stdout)
        d = data["output"]
        raw = d["raw_lambda1"]
        lo = [x for x in raw if x < 60]
        lo_min = min(lo) if lo else d["lambda1_min"]
        lo_mean = sum(lo)/len(lo) if lo else 0
        hi_frac = sum(1 for x in raw if x >= 60) / len(raw)
        return lo_min, lo_mean, hi_frac
    except Exception as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        return None, None, None

defaults = {
    "damping": "0.1",
    "coupling": "1.0",
    "length": "1.0",
    "mass": "1.0",
}

rows = []
def process_one(arg):
    pname, val = arg
    lo_min, lo_mean, hi_frac = run_one(pname, val, defaults)
    if lo_min is not None:
        print(f"  {pname}={val}: low_min={lo_min:.4f} low_mean={lo_mean:.3f} hi_frac={hi_frac:.3f}", flush=True)
        return {
            "parameter": pname,
            "value": val,
            "lowband_lambda_min": f"{lo_min:.6f}",
            "lowband_lambda_mean": f"{lo_mean:.4f}",
            "highband_frac": f"{hi_frac:.4f}",
        }
    print(f"  {pname}={val}: FAILED", flush=True)
    return None

WORKERS = int(os.environ.get("SWEEP_WORKERS", "6"))
jobs = [(pname, val) for pname, pinfo in PARAMS.items() for val in pinfo["vals"]]
print(f"Running {len(jobs)} sweep points with {WORKERS} workers (T={STEPS*0.01}s)...")
with ThreadPoolExecutor(max_workers=WORKERS) as ex:
    rows = [r for r in ex.map(process_one, jobs) if r is not None]

with open(OUT, "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=["parameter", "value", "lowband_lambda_min", "lowband_lambda_mean", "highband_frac"])
    w.writeheader()
    w.writerows(rows)
print(f"\nWrote {len(rows)} points to {OUT}")
