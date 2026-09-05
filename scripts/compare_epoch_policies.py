#!/usr/bin/env python3
"""Comparative analysis of the two epoch-cap policies under the metastability
finding (docs/design_note_metastability.md).

  (a) Random-IC resampling capped at window = 15 s (steps = 1500), the strict
      worst-case envelope from the escape CDF (min escape ~17.6 s).
  (b) Deterministic cold-start IC (theta = 0.1, 0.2, 0.3) at window = 100 s
      (steps = 10000), inside its measured ~127 s escape envelope.

Writes under results_v3/compare/:
  lambda_min_series_t15s.csv          (a: 10 trials x 1000 samples, default pt)
  pendulum_robustness_t15s.csv        (a: same grid as committed sweep, 500 smp)
  deterministic_ic_lambda1.csv        (b: lambda1 at 100 s and 15 s per config)
  policy_comparison.csv               merged table + per-config escape envelope
  policy_comparison_summary.json

Run:  python3 scripts/compare_epoch_policies.py
"""
import csv
import json
import os
import subprocess
import sys
import time

import numpy as np

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CLI = os.path.join(ROOT, "core_v2", "target", "release", "cli_v2")
OUT = os.path.join(ROOT, "results_v3", "compare")
os.makedirs(OUT, exist_ok=True)

DEFAULTS = {"mass": "1.0", "length": "1.0", "damping": "0.1", "coupling": "0.5"}
PARAMS = {
    "damping":  {"flag": "--damping",  "vals": [0.055, 0.06, 0.07, 0.08, 0.09, 0.10, 0.2, 0.4]},
    "coupling": {"flag": "--coupling", "vals": [0.2, 0.3, 0.4, 0.5, 0.6, 0.8, 1.0]},
    "length":   {"flag": "--length",   "vals": [0.5, 1.0, 2.0, 4.0]},
    "mass":     {"flag": "--mass",     "vals": [0.5, 1.0, 1.5, 2.0]},
}
SAMPLES_A = 500
STEPS_A = 1500            # 15 s window (strict cap)
STEPS_B = 10000           # 100 s window (deterministic-IC envelope)
BOUND = 50.0
DT = 2e-3
TMAX = 400.0

G = 9.80665
N = 3
BOOST = 3.0
KICK_TH = 0.5


def flags_for(pname, val):
    out = []
    if pname is None:
        for p, v in DEFAULTS.items():
            out += [PARAMS[p]["flag"], v]
        return out
    for p, v in DEFAULTS.items():
        if p == pname:
            out += [PARAMS[p]["flag"], str(val)]
        else:
            out += [PARAMS[p]["flag"], v]
    return out


def cli(args):
    res = subprocess.run([CLI] + args, capture_output=True, text=True, timeout=600)
    if res.returncode != 0:
        raise RuntimeError(res.stderr)
    return json.loads(res.stdout)["output"]


def lambda_min_a(pname, val):
    d = cli(["lyapunov-attractor"] + flags_for(pname, val) +
            ["--samples", str(SAMPLES_A), "--steps", str(STEPS_A)])
    raw = d["raw_lambda1"]
    lo = [x for x in raw if x < 60.0]
    return {
        "lambda_min_t15": min(lo) if lo else d["lambda1_min"],
        "lambda_mean_t15": sum(lo) / len(lo) if lo else 0.0,
        "highband_frac_t15": 1.0 - len(lo) / len(raw),
    }


def lambda1_b(pname, val):
    base = flags_for(pname, val)
    l100 = cli(["lyapunov"] + base + ["--steps", str(STEPS_B)])["lambda1"]
    l15 = cli(["lyapunov"] + base + ["--steps", str(STEPS_A)])["lambda1"]
    return {"lambda1_det_100s": l100, "lambda1_det_15s": l15}


def run():
    t0 = time.time()
    log = lambda *a: print(*a, flush=True)

    # ---- (a) lambda_min series at default point, 15 s window --------------
    series = []
    for trial in range(1, 11):
        d = cli(["lyapunov-attractor"] + flags_for(None, None) +
                ["--samples", "1000", "--steps", str(STEPS_A)])
        # flags_for(None,None) returns defaults; guard below
        raw = d["raw_lambda1"]
        lo = [x for x in raw if x < 60.0]
        series.append({
            "trial": trial,
            "lambda_min_nats_per_s": f"{min(raw):.12f}",
            "lambda_max_nats_per_s": f"{max(raw):.12f}",
            "highband_frac": f"{1.0 - len(lo) / len(raw):.4f}",
            "lowband_mean_nats_per_s":
                f"{(sum(lo) / len(lo) if lo else 0.0):.12f}",
        })
        log(f"  series trial {trial}: min={min(raw):.4f} mean(low)="
            f"{sum(lo) / len(lo) if lo else 0:.4f}")
    with open(os.path.join(OUT, "lambda_min_series_t15s.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(series[0].keys()))
        w.writeheader(); w.writerows(series)

    # ---- (a) + (b) per-config grid -----------------------------------------
    rows = []
    configs = []
    for pname, pinfo in PARAMS.items():
        for val in pinfo["vals"]:
            configs.append((pname, val))
    # deterministic default point too
    configs.append((None, None))

    for (pname, val) in configs:
        save = flags_for(pname, val)
        eff_pname = pname
        if pname is None:
            # default point: reuse committed param identity as "default"
            eff_pname = "default"
            a = lambda_min_a(None, None)
        else:
            a = lambda_min_a(pname, val)
        b = lambda1_b(pname, val)
        esc = escape_time_for(pname, val)
        rows.append({
            "parameter": eff_pname if pname is not None else "default",
            "value": "" if pname is None else val,
            **a,
            **b,
            "escape_time_s": None if esc is None else round(esc, 1),
            "margin_100s_ok": (esc is not None and esc > 100.0),
        })
        log(f"  {rows[-1]['parameter']} {rows[-1]['value']}: "
            f"(a)min={a['lambda_min_t15']:.4f} mean={a['lambda_mean_t15']:.4f} "
            f"(b)l1@100={b['lambda1_det_100s']:.4f} esc={esc}")

    fields = ["parameter", "value", "lambda_min_t15", "lambda_mean_t15",
              "highband_frac_t15", "lambda1_det_100s", "lambda1_det_15s",
              "escape_time_s", "margin_100s_ok"]
    with open(os.path.join(OUT, "policy_comparison.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader(); w.writerows(rows)

    # ---- merged deterministic lambda1 table --------------------------------
    with open(os.path.join(OUT, "deterministic_ic_lambda1.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=["parameter", "value",
                                          "lambda1_det_100s", "lambda1_det_15s"])
        w.writeheader()
        for r in rows:
            w.writerow({"parameter": r["parameter"], "value": r["value"],
                        "lambda1_det_100s": r["lambda1_det_100s"],
                        "lambda1_det_15s": r["lambda1_det_15s"]})

    # ---- summary ------------------------------------------------------------
    summary = {"steps_a": STEPS_A, "steps_b": STEPS_B, "rows": len(rows),
               "duration_s": round(time.time() - t0, 1)}
    log(f"\nWrote {len(series)} series rows + {len(rows)} configs to {OUT} "
        f"({summary['duration_s']}s)")
    with open(os.path.join(OUT, "policy_comparison_summary.json"), "w") as f:
        json.dump(summary, f, indent=2)


def escape_time_for(pname, val):
    m = float(DEFAULTS["mass"]) if pname != "mass" else float(val)
    L = float(DEFAULTS["length"]) if pname != "length" else float(val)
    b = float(DEFAULTS["damping"]) if pname != "damping" else float(val)
    c = float(DEFAULTS["coupling"]) if pname != "coupling" else float(val)
    if pname is None:
        m, L, b, c = 1.0, 1.0, 0.1, 0.5

    def deriv(xx):
        th = xx[:N]; om = xx[N:]
        d = np.zeros_like(xx)
        d[N:] = -b * om
        inertia = m * L * L
        dmin = min(L, L)
        for i in range(N):
            tg = G * (m * 2.0) * (L / 2.0) * np.sin(th[i])
            tc = c * ((th[i] - th[i - 1]) / dmin) * 0.1 if i >= 1 else 0.0
            d[N + i] += (tg + tc) / inertia
            d[i] = om[i]
        return d

    x = np.array([0.1, 0.2, 0.3, 0.0, 0.0, 0.0])
    steps = int(TMAX / DT)
    first = None
    for step in range(steps):
        s = x.copy()
        if np.sum(np.abs(s[N:])) < KICK_TH:
            s[N] += BOOST
        k1 = deriv(s); s2 = s + 0.5 * DT * k1; k2 = deriv(s2)
        s3 = s + 0.5 * DT * k2; k3 = deriv(s3); s4 = s + DT * k3; k4 = deriv(s4)
        x = s + DT / 6.0 * (k1 + 2 * k2 + 2 * k3 + k4)
        if first is None and np.max(np.abs(x[N:])) > BOUND:
            first = (step + 1) * DT
    return first


if __name__ == "__main__":
    sys.exit(run())