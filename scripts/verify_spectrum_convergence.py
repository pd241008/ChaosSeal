#!/usr/bin/env python3
"""Verify finite-horizon convergence of the full Lyapunov spectrum.

Runs LyapunovSpectrum at increasing horizons (T=500, 1000, 2000, 4000, 8000 s)
and checks convergence of lambda_1, lambda_2, lambda_3, and KS entropy.
"""
import json
import subprocess
import sys
import os
import numpy as np
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CLI = ROOT / "core_v2" / "target" / "release" / "cli_v2"

# Test configurations: (horizon_s, steps) pairs
HORIZONS = [
    (200, 20000),
    (500, 50000),
    (1000, 100000),
    (2000, 200000),
    (4000, 400000),
]

SAMPLES = 100  # reduced for long horizons; can increase
PENDULUMS = 3
MASS = 1.0
LENGTH = 1.0
DAMPING = 0.1
COUPLING = 1.0

def run_spectrum(steps, samples):
    cmd = [
        str(CLI), "lyapunov-spectrum",
        "--pendulums", str(PENDULUMS),
        "--mass", str(MASS),
        "--length", str(LENGTH),
        "--damping", str(DAMPING),
        "--coupling", str(COUPLING),
        "--steps", str(steps),
        "--samples", str(samples),
    ]
    print(f"  Running: T={steps*0.01:.0f}s, samples={samples}...")
    res = subprocess.run(cmd, capture_output=True, text=True, timeout=1800)
    if res.returncode != 0:
        raise RuntimeError(f"CLI failed: {res.stderr}")
    return json.loads(res.stdout)["output"]

def stats_from_spectra(spectra):
    """Compute mean, std, min, max from raw spectra."""
    arr = np.array(spectra)
    return {
        "mean": arr.mean(axis=0).tolist(),
        "std": arr.std(axis=0, ddof=1).tolist(),
        "min": arr.min(axis=0).tolist(),
        "max": arr.max(axis=0).tolist(),
    }

def main():
    print("=" * 70)
    print("Lyapunov Spectrum Finite-Horizon Convergence Verification")
    print("=" * 70)
    print(f"Parameters: N={PENDULUMS}, m={MASS}, L={LENGTH}, b={DAMPING}, c={COUPLING}")
    print(f"Samples per horizon: {SAMPLES}")
    print()

    results = {}
    for T, steps in HORIZONS:
        # Reduce samples for very long horizons to keep runtime reasonable
        s = SAMPLES if T <= 2000 else min(SAMPLES, 100)
        out = run_spectrum(steps, s)
        results[T] = out
        spec = out["raw_spectra"]
        st = stats_from_spectra(spec)
        print(f"\nT = {T} s (steps={steps}, samples={s}):")
        print(f"  lambda_1: mean={st['mean'][0]:.4f} ± {st['std'][0]:.4f}  range=[{st['min'][0]:.4f}, {st['max'][0]:.4f}]")
        print(f"  lambda_2: mean={st['mean'][1]:.4f} ± {st['std'][1]:.4f}  range=[{st['min'][1]:.4f}, {st['max'][1]:.4f}]")
        print(f"  lambda_3: mean={st['mean'][2]:.4f} ± {st['std'][2]:.4f}  range=[{st['min'][2]:.4f}, {st['max'][2]:.4f}]")
        ks_vals = [sum(x for x in s if x > 0) for s in spec]
        print(f"  KS:       mean={np.mean(ks_vals):.4f} ± {np.std(ks_vals, ddof=1):.4f}  range=[{np.min(ks_vals):.4f}, {np.max(ks_vals):.4f}]")
        print(f"  dt_bound(256-bit, lambda1_min): {out['dt_bound_from_l1_min_s']:.1f} s")
        print(f"  dt_bound(256-bit, KS_min):      {out['dt_bound_from_ks_min_s']:.1f} s")

    # Convergence analysis
    print("\n" + "=" * 70)
    print("CONVERGENCE ANALYSIS")
    print("=" * 70)
    
    # Compare consecutive horizons
    for i in range(1, len(HORIZONS)):
        T_prev, _ = HORIZONS[i-1]
        T_curr, _ = HORIZONS[i]
        prev = results[T_prev]["raw_spectra"]
        curr = results[T_curr]["raw_spectra"]
        
        prev_mean = np.mean(prev, axis=0)
        curr_mean = np.mean(curr, axis=0)
        rel_diff = np.abs(curr_mean - prev_mean) / np.maximum(np.abs(prev_mean), 1e-6)
        
        print(f"\n{T_prev}s -> {T_curr}s relative changes:")
        print(f"  lambda_1: {rel_diff[0]*100:.2f}%")
        print(f"  lambda_2: {rel_diff[1]*100:.2f}%")
        print(f"  lambda_3: {rel_diff[2]*100:.2f}%")
        
        prev_ks = [sum(x for x in s if x > 0) for s in prev]
        curr_ks = [sum(x for x in s if x > 0) for s in curr]
        ks_rel = abs(np.mean(curr_ks) - np.mean(prev_ks)) / max(abs(np.mean(prev_ks)), 1e-6)
        print(f"  KS:       {ks_rel*100:.2f}%")

    # Final converged values (longest horizon)
    T_final = HORIZONS[-1][0]
    final_spec = results[T_final]["raw_spectra"]
    final_st = stats_from_spectra(final_spec)
    final_ks = [sum(x for x in s if x > 0) for s in final_spec]
    
    print("\n" + "=" * 70)
    print(f"CONVERGED VALUES (T={T_final}s, samples={len(final_spec)})")
    print("=" * 70)
    print(f"lambda_1: {final_st['mean'][0]:.4f} ± {final_st['std'][0]:.4f}  (min={final_st['min'][0]:.4f}, max={final_st['max'][0]:.4f})")
    print(f"lambda_2: {final_st['mean'][1]:.4f} ± {final_st['std'][1]:.4f}  (min={final_st['min'][1]:.4f}, max={final_st['max'][1]:.4f})")
    print(f"lambda_3: {final_st['mean'][2]:.4f} ± {final_st['std'][2]:.4f}  (min={final_st['min'][2]:.4f}, max={final_st['max'][2]:.4f})")
    print(f"KS:       {np.mean(final_ks):.4f} ± {np.std(final_ks, ddof=1):.4f}  (min={np.min(final_ks):.4f}, max={np.max(final_ks):.4f})")
    print(f"\nlambda_2 > 0 in {results[T_final]['l2_positive_samples']}/{len(final_spec)} samples")
    print(f"lambda_3 > 0 in {results[T_final]['l3_positive_samples']}/{len(final_spec)} samples")
    
    # Entropy bound comparison
    l1_min = final_st['min'][0]
    ks_min = np.min(final_ks)
    dt_l1 = 256 * np.log(2) / l1_min if l1_min > 0 else float('inf')
    dt_ks = 256 * np.log(2) / ks_min if ks_min > 0 else float('inf')
    
    print(f"\n256-bit entropy accumulation bounds:")
    print(f"  Using lambda_1 only (conservative): dt_bound = {dt_l1:.1f} s")
    print(f"  Using KS entropy (full spectrum):   dt_bound = {dt_ks:.1f} s")
    print(f"  Improvement factor: {dt_l1/dt_ks:.2f}x")
    
    # Save raw data
    import csv
    out_dir = ROOT / "results_v3" / "convergence"
    out_dir.mkdir(parents=True, exist_ok=True)
    
    for T, _ in HORIZONS:
        out = results[T]
        csv_path = out_dir / f"spectrum_T{T}s.csv"
        with open(csv_path, 'w', newline='') as f:
            w = csv.writer(f)
            w.writerow(["sample", "lambda_1", "lambda_2", "lambda_3", "KS"])
            for i, spec in enumerate(out["raw_spectra"]):
                ks = sum(x for x in spec if x > 0)
                w.writerow([i+1, spec[0], spec[1], spec[2], ks])
        print(f"  Saved {csv_path}")
    
    # Summary CSV
    summary_path = out_dir / "convergence_summary.csv"
    with open(summary_path, 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(["horizon_s", "lambda1_mean", "lambda1_std", "lambda1_min", "lambda1_max",
                    "lambda2_mean", "lambda2_std", "lambda2_min", "lambda2_max",
                    "lambda3_mean", "lambda3_std", "lambda3_min", "lambda3_max",
                    "ks_mean", "ks_std", "ks_min", "ks_max",
                    "dt_bound_l1_min", "dt_bound_ks_min"])
        for T, _ in HORIZONS:
            out = results[T]
            spec = out["raw_spectra"]
            st = stats_from_spectra(spec)
            ks_vals = [sum(x for x in s if x > 0) for s in spec]
            w.writerow([
                T,
                st['mean'][0], st['std'][0], st['min'][0], st['max'][0],
                st['mean'][1], st['std'][1], st['min'][1], st['max'][1],
                st['mean'][2], st['std'][2], st['min'][2], st['max'][2],
                np.mean(ks_vals), np.std(ks_vals, ddof=1), np.min(ks_vals), np.max(ks_vals),
                out['dt_bound_from_l1_min_s'], out['dt_bound_from_ks_min_s'],
            ])
    print(f"  Saved {summary_path}")
    
    # Check convergence criterion: relative change < 5% for last step
    T_prev, _ = HORIZONS[-2]
    T_curr, _ = HORIZONS[-1]
    prev_spec = results[T_prev]["raw_spectra"]
    curr_spec = results[T_curr]["raw_spectra"]
    prev_mean = np.mean(prev_spec, axis=0)
    curr_mean = np.mean(curr_spec, axis=0)
    rel_diff = np.abs(curr_mean - prev_mean) / np.maximum(np.abs(prev_mean), 1e-6)
    
    converged = all(d < 0.05 for d in rel_diff)
    print(f"\n{'=' * 70}")
    if converged:
        print("✓ CONVERGED: All exponents stable within 5% between last two horizons")
    else:
        print("✗ NOT CONVERGED: Some exponents changing >5% between last two horizons")
        for j, d in enumerate(rel_diff):
            status = "✓" if d < 0.05 else "✗"
            print(f"  lambda_{j+1}: {d*100:.2f}% change {status}")
    
    return 0 if converged else 1

if __name__ == "__main__":
    sys.exit(main())