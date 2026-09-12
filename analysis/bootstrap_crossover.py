#!/usr/bin/env python3
"""Bootstrap confidence intervals on the crossover point (|R|=64-128).

Analyzes the v3 R-sweep data across 5 seeds to compute bootstrap CIs
for the goodput crossover where CEP goodput = BPSec goodput.
"""
import json
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from pathlib import Path
import csv

RESULTS = Path(__file__).resolve().parent.parent / "results_v3"
FIG = RESULTS / "figures"
FIG.mkdir(exist_ok=True)

# Load all R-sweep data
def load_r_sweep_data():
    """Load goodput for chaosseal and bpsec across seeds and R values."""
    data = {}
    for seed in range(1, 6):
        for r in [1, 2, 4, 8, 16, 32, 64, 128, 256, 512]:
            fname = f"v3-rsweep-seed{seed}-r{r:04d}.json"
            fpath = RESULTS / fname
            if not fpath.exists():
                continue
            with open(fpath) as f:
                d = json.load(f)
            
            # Extract goodput
            def goodput(name):
                blk = d["baselines"].get(name, {})
                if name == "bpsec":
                    return blk["payload_bytes"] * 8 / blk["transfer_sec"] / 1e6
                dt = blk.get("data_transmission", {})
                if not dt:
                    return None
                bits = dt["payload_bytes"] * 8
                sec = dt["transfer_sec"] + dt.get("crypto_wallclock_us", 0) / 1e6
                return bits / sec / 1e6
            
            cs_gp = goodput("chaosseal")
            bp_gp = goodput("bpsec")
            if cs_gp is not None and bp_gp is not None:
                data.setdefault((seed, r), {})["chaosseal"] = cs_gp
                data.setdefault((seed, r), {})["bpsec"] = bp_gp
    return data

def compute_crossover(r_vals, cs_gps, bp_gps):
    """Find crossover point where CEP goodput = BPSec goodput.
    Returns crossover revocation fraction (or None if no crossover)."""
    ratios = np.array(cs_gps) / np.array(bp_gps)
    # Find where ratio crosses 1.0
    for i in range(len(r_vals) - 1):
        if ratios[i] >= 1.0 and ratios[i+1] < 1.0:
            # Linear interpolation
            r1, r2 = r_vals[i], r_vals[i+1]
            rat1, rat2 = ratios[i], ratios[i+1]
            frac = (1.0 - rat1) / (rat2 - rat1)
            return r1 + frac * (r2 - r1)
    if ratios[0] < 1.0:
        return None  # Already below at R=1
    if ratios[-1] > 1.0:
        return r_vals[-1]  # Still above at max R
    return None

def bootstrap_crossover(data, n_bootstrap=10000):
    """Bootstrap CI on crossover point."""
    seeds = list(range(1, 6))
    r_vals = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512]
    
    # Organize by R: list of (chaosseal, bpsec) pairs per seed
    by_r = {}
    for (seed, r), vals in data.items():
        if r not in by_r:
            by_r[r] = []
        by_r[r].append((vals["chaosseal"], vals["bpsec"]))
    
    boot_crossovers = []
    
    for _ in range(n_bootstrap):
        # Resample seeds with replacement
        boot_seeds = np.random.choice(seeds, size=len(seeds), replace=True)
        boot_cs = []
        boot_bp = []
        
        for r in r_vals:
            # For each R, resample from available seed data
            if r not in by_r:
                continue
            idx = np.random.randint(len(by_r[r]))
            cs, bp = by_r[r][idx]
            boot_cs.append(cs)
            boot_bp.append(bp)
        
        if len(boot_cs) == len(r_vals):
            cross = compute_crossover(r_vals, boot_cs, boot_bp)
            if cross is not None:
                boot_crossovers.append(cross)
    
    return np.array(boot_crossovers)

def main():
    print("=" * 70)
    print("Bootstrap CI on Crossover Point (|R|=64-128)")
    print("=" * 70)
    
    data = load_r_sweep_data()
    print(f"Loaded {len(data)} (seed, R) pairs")
    
    # Group by seed and R
    seeds = list(range(1, 6))
    r_vals = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512]
    
    # Print raw goodput table
    print("\nRaw goodput per seed and R (Mbps):")
    print(f"{'Seed':>4} " + " ".join(f"R={r:<4}" for r in r_vals))
    for seed in seeds:
        row = [f"{seed:>4}"]
        for r in r_vals:
            if (seed, r) in data:
                cs = data[(seed, r)]["chaosseal"]
                bp = data[(seed, r)]["bpsec"]
                row.append(f"{cs:.3f}/{bp:.3f}")
            else:
                row.append("  -    ")
        print(" ".join(row))
    
    # Mean goodput per R
    print("\nMean goodput per R (Mbps):")
    print(f"{'R':>4} {'chaosseal_mean':>14} {'chaosseal_std':>14} {'bpsec_mean':>14} {'bpsec_std':>14} {'ratio':>8}")
    for r in r_vals:
        cs_vals = [data[(s, r)]["chaosseal"] for s in seeds if (s, r) in data]
        bp_vals = [data[(s, r)]["bpsec"] for s in seeds if (s, r) in data]
        if cs_vals and bp_vals:
            cs_mean, cs_std = np.mean(cs_vals), np.std(cs_vals, ddof=1)
            bp_mean, bp_std = np.mean(bp_vals), np.std(bp_vals, ddof=1)
            ratio = cs_mean / bp_mean
            print(f"{r:>4} {cs_mean:>14.4f} {cs_std:>14.4f} {bp_mean:>14.4f} {bp_std:>14.4f} {ratio:>8.4f}")
    
    # Compute crossover per seed
    print("\nCrossover per seed:")
    seed_crossovers = {}
    for seed in seeds:
        cs_gps = [data[(seed, r)]["chaosseal"] for r in r_vals if (seed, r) in data]
        bp_gps = [data[(seed, r)]["bpsec"] for r in r_vals if (seed, r) in data]
        if len(cs_gps) == len(r_vals):
            cross = compute_crossover(r_vals, cs_gps, bp_gps)
            seed_crossovers[seed] = cross
            print(f"  Seed {seed}: {cross:.2f} revoked ({cross/1024*100:.1f}%)")
    
    # Bootstrap
    print("\nRunning bootstrap (10,000 iterations)...")
    np.random.seed(42)
    boot_crossovers = bootstrap_crossover(data, n_bootstrap=10000)
    
    if len(boot_crossovers) > 0:
        print(f"  Successful bootstraps: {len(boot_crossovers)}/10000")
        print(f"  Mean crossover: {np.mean(boot_crossovers):.2f} revoked ({np.mean(boot_crossovers)/1024*100:.2f}%)")
        print(f"  Std:  {np.std(boot_crossovers):.2f}")
        print(f"  95% CI: [{np.percentile(boot_crossovers, 2.5):.2f}, {np.percentile(boot_crossovers, 97.5):.2f}]")
        print(f"  90% CI: [{np.percentile(boot_crossovers, 5):.2f}, {np.percentile(boot_crossovers, 95):.2f}]")
        print(f"  Min/Max: [{np.min(boot_crossovers):.2f}, {np.max(boot_crossovers):.2f}]")
        
        # Also in percentage
        boot_pct = boot_crossovers / 1024 * 100
        print(f"\n  Crossover as % of swarm:")
        print(f"    Mean: {np.mean(boot_pct):.2f}%")
        print(f"    95% CI: [{np.percentile(boot_pct, 2.5):.2f}%, {np.percentile(boot_pct, 97.5):.2f}%]")
        print(f"    90% CI: [{np.percentile(boot_pct, 5):.2f}%, {np.percentile(boot_pct, 95):.2f}%]")
        
        # Save bootstrap distribution
        with open(RESULTS / "v3_bootstrap_crossover.csv", "w", newline="") as f:
            w = csv.writer(f)
            w.writerow(["bootstrap_iteration", "crossover_R", "crossover_pct"])
            for i, c in enumerate(boot_crossovers):
                w.writerow([i+1, c, c/1024*100])
        print(f"\nSaved bootstrap samples to {RESULTS / 'v3_bootstrap_crossover.csv'}")
        
        # Plot bootstrap distribution
        fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5))
        
        ax1.hist(boot_crossovers, bins=50, color="tab:blue", edgecolor="k", alpha=0.7)
        ax1.axvline(np.mean(boot_crossovers), color="k", ls="--", lw=1.5, label=f"Mean={np.mean(boot_crossovers):.1f}")
        ax1.axvline(64, color="r", ls=":", lw=1, label="R=64")
        ax1.axvline(128, color="r", ls=":", lw=1, label="R=128")
        ax1.set_xlabel("Crossover |R| (revoked nodes)")
        ax1.set_ylabel("Bootstrap count")
        ax1.set_title("Bootstrap Distribution of Crossover Point")
        ax1.legend(fontsize=8)
        ax1.grid(True, alpha=0.3)
        
        ax2.hist(boot_pct, bins=50, color="tab:orange", edgecolor="k", alpha=0.7)
        ax2.axvline(np.mean(boot_pct), color="k", ls="--", lw=1.5, label=f"Mean={np.mean(boot_pct):.2f}%")
        ax2.axvline(64/1024*100, color="r", ls=":", lw=1, label="6.25%")
        ax2.axvline(128/1024*100, color="r", ls=":", lw=1, label="12.5%")
        ax2.set_xlabel("Crossover revocation fraction (%)")
        ax2.set_ylabel("Bootstrap count")
        ax2.set_title("Bootstrap Distribution of Crossover Fraction")
        ax2.legend(fontsize=8)
        ax2.grid(True, alpha=0.3)
        
        fig.tight_layout()
        fig.savefig(FIG / "v3_bootstrap_crossover.pdf")
        plt.close(fig)
        print(f"Saved {FIG / 'v3_bootstrap_crossover.pdf'}")
        
        # Plot per-seed crossover
        fig, ax = plt.subplots(figsize=(10, 5))
        seed_nums = list(seed_crossovers.keys())
        cross_vals = list(seed_crossovers.values())
        ax.bar([str(s) for s in seed_nums], cross_vals, color="tab:blue", alpha=0.7, edgecolor="k")
        ax.axhline(np.mean(cross_vals), color="k", ls="--", lw=1.5, label=f"Mean={np.mean(cross_vals):.1f}")
        ax.axhline(64, color="r", ls=":", lw=1.5, label="R=64")
        ax.axhline(128, color="r", ls=":", lw=1.5, label="R=128")
        ax.set_xlabel("Seed")
        ax.set_ylabel("Crossover |R|")
        ax.set_title("Per-Seed Crossover Point (R where CEP goodput = BPSec)")
        ax.legend()
        ax.grid(True, alpha=0.3)
        fig.tight_layout()
        fig.savefig(FIG / "v3_per_seed_crossover.pdf")
        plt.close(fig)
        print(f"Saved {FIG / 'v3_per_seed_crossover.pdf'}")
        
        # Summary stats CSV
        with open(RESULTS / "v3_crossover_bootstrap_stats.csv", "w", newline="") as f:
            w = csv.writer(f)
            w.writerow(["statistic", "crossover_R", "crossover_pct"])
            w.writerow(["mean", np.mean(boot_crossovers), np.mean(boot_pct)])
            w.writerow(["std", np.std(boot_crossovers), np.std(boot_pct)])
            w.writerow(["ci95_lower", np.percentile(boot_crossovers, 2.5), np.percentile(boot_pct, 2.5)])
            w.writerow(["ci95_upper", np.percentile(boot_crossovers, 97.5), np.percentile(boot_pct, 97.5)])
            w.writerow(["ci90_lower", np.percentile(boot_crossovers, 5), np.percentile(boot_pct, 5)])
            w.writerow(["ci90_upper", np.percentile(boot_crossovers, 95), np.percentile(boot_pct, 95)])
            w.writerow(["min", np.min(boot_crossovers), np.min(boot_pct)])
            w.writerow(["max", np.max(boot_crossovers), np.max(boot_pct)])
            for seed in seeds:
                if seed in seed_crossovers:
                    w.writerow([f"seed{seed}", seed_crossovers[seed], seed_crossovers[seed]/1024*100])
        print(f"Saved summary stats to {RESULTS / 'v3_crossover_bootstrap_stats.csv'}")
        
    else:
        print("  No successful bootstraps!")
    
    print("\nDone.")

if __name__ == "__main__":
    main()