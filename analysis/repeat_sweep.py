#!/usr/bin/env python3
"""
repeat_sweep.py — run the R-sweep across multiple RNG seeds and compute
mean/std for error bars, instead of the current single-seed numbers.

Usage:
  Run your existing netsim binary once per seed, e.g.:
    for seed in 1 2 3 4 5; do
      ./netsim --sweep --seed $seed --out results/run_seed${seed}.csv
    done
  Then:
    python3 repeat_sweep.py results/run_seed*.csv > results/sweep_stats.csv

This does NOT invent data — it only aggregates CSVs you already produce.
If you only have one seed's worth of runs right now, this script will
just report n=1 (std=NaN) so you can see exactly what's still missing
before it goes in the paper.
"""
import sys
import csv
import statistics as st
from collections import defaultdict

def load(path):
    rows = []
    with open(path) as f:
        for row in csv.DictReader(f):
            rows.append(row)
    return rows

def main(paths):
    # key: (Scenario, Protocol) -> list of goodput values across seeds
    by_key = defaultdict(list)
    for p in paths:
        for row in load(p):
            key = (row["Scenario"], row["Protocol"])
            by_key[key].append(float(row["Goodput_Mbps"]))

    print("Scenario,Protocol,N,Mean_Goodput_Mbps,Std_Goodput_Mbps,Min,Max")
    for (scenario, protocol), vals in sorted(by_key.items()):
        n = len(vals)
        mean = st.mean(vals)
        std = st.stdev(vals) if n > 1 else float("nan")
        print(f"{scenario},{protocol},{n},{mean:.4f},{std:.4f},{min(vals):.4f},{max(vals):.4f}")

    n_seeds = max(len(v) for v in by_key.values())
    if n_seeds < 3:
        print(f"\n# WARNING: only {n_seeds} seed(s) found. Need >=3-5 repeats "
              f"before reporting std/error bars credibly in the paper.",
              file=sys.stderr)

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    main(sys.argv[1:])
