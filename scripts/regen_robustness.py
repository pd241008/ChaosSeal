#!/usr/bin/env python3
"""Regenerate pendulum-robustness sweep data from the actual CLI binary.

Runs lyapunov-attractor at each parameter point, extracts the low-band λ_min
(physical minimum only, excluding the ~15% blow-up artifact band), and writes
a committed CSV so v4_generalization.pendulum_robustness() is reproducible.
"""
import subprocess, json, csv, sys, os

CLI = os.path.join(os.path.dirname(__file__), "..", "core_v2", "target", "release", "cli_v2")
SAMPLES = 500
STEPS = 10000
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
    "coupling": "0.5",
    "length": "1.0",
    "mass": "1.0",
}

rows = []
for pname, pinfo in PARAMS.items():
    print(f"Running {pname} sweep ({len(pinfo['vals'])} points)...")
    for val in pinfo["vals"]:
        lo_min, lo_mean, hi_frac = run_one(pname, val, defaults)
        print(f"  {pname}={val}: low_min={lo_min:.4f} low_mean={lo_mean:.3f} hi_frac={hi_frac:.3f}" if lo_min else f"  {pname}={val}: FAILED")
        rows.append({
            "parameter": pname,
            "value": val,
            "lowband_lambda_min": f"{lo_min:.6f}" if lo_min else "",
            "lowband_lambda_mean": f"{lo_mean:.4f}" if lo_mean else "",
            "highband_frac": f"{hi_frac:.4f}" if hi_frac else "",
        })

with open(OUT, "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=["parameter", "value", "lowband_lambda_min", "lowband_lambda_mean", "highband_frac"])
    w.writeheader()
    w.writerows(rows)
print(f"\nWrote {len(rows)} points to {OUT}")
