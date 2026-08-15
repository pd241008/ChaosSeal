#!/usr/bin/env python3
"""Generate the 3 paper figures from /results/*.json.

Figures (vector PDF into analysis/figures/):
    1. bee_size_vs_r.pdf    BEE ciphertext size vs |R|
    2. resync_latency.pdf   Resynchronization latency distribution
    3. throughput.pdf       Throughput comparison (ChaosSeal vs TLS 1.3 vs BPSec)

Never recomputes protocol logic; reads the same JSON fields as stats.py.

Usage:
    python3 figures.py [results_dir]
"""
from __future__ import annotations

import os
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

from stats import (
    RESULTS_DIR,
    bee_sizes,
    link_latency_samples_ms,
    load_results,
    main_runs,
    throughput_mbps,
)

FIGURES_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "figures")

SERIF = ["DejaVu Serif"]
plt.rcParams["font.family"] = "serif"
plt.rcParams["font.serif"] = SERIF
plt.rcParams["figure.dpi"] = 150


def _save(fig, name: str) -> str:
    os.makedirs(FIGURES_DIR, exist_ok=True)
    path = os.path.join(FIGURES_DIR, name)
    fig.savefig(path, format="pdf", bbox_inches="tight")
    plt.close(fig)
    return path


def figure1_bee_size(runs: list[dict]) -> str:
    sizes = bee_sizes(runs)
    if not sizes:
        raise RuntimeError("no BEE size data in results")
    rs = sorted(sizes)
    ys = [float(np.mean(sizes[r])) for r in rs]

    fig, ax = plt.subplots(figsize=(4.5, 3.4))
    ax.plot(rs, ys, "o-", color="#1f77b4", markersize=5, linewidth=1.5)
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel("Number of revoked receivers $|R|$")
    ax.set_ylabel("BEE ciphertext size (bytes)")
    ax.set_title("BEE ciphertext size vs $|R|$  ($N=1024$)")
    return _save(fig, "bee_size_vs_r.pdf")


def figure2_resync_latency(runs: list[dict]) -> str:
    lats = link_latency_samples_ms(main_runs(runs))
    if not lats:
        raise RuntimeError("no latency samples in main runs")
    arr = np.array(lats)

    fig, ax = plt.subplots(figsize=(4.5, 3.4))
    ax.hist(arr, bins=40, color="#2ca02c", edgecolor="black", linewidth=0.5)
    for p, style, label in (
        (50, "--", "median"),
        (95, "-.", "p95"),
    ):
        v = np.percentile(arr, p)
        ax.axvline(v, color="black", linestyle=style, linewidth=1.0, label=f"{label} = {v:.2f} ms")
    ax.set_xlabel("Resynchronization link latency (ms)")
    ax.set_ylabel("Count")
    ax.set_title("Resynchronization link latency distribution")
    ax.legend(frameon=False)
    return _save(fig, "resync_latency.pdf")


def figure3_throughput(runs: list[dict]) -> str:
    tp = throughput_mbps(main_runs(runs))
    if not tp or not any(tp.values()):
        raise RuntimeError("no throughput data in main runs")
    names = ["chaosseal", "tls13", "bpsec"]
    labels = ["ChaosSeal", "TLS 1.3", "BPSec"]
    means = [np.mean(tp[n]) for n in names]
    mins = [np.min(tp[n]) for n in names]
    maxs = [np.max(tp[n]) for n in names]

    fig, ax = plt.subplots(figsize=(4.5, 3.4))
    x = np.arange(len(names))
    colors = ["#1f77b4", "#ff7f0e", "#d62728"]
    bars = ax.bar(x, means, width=0.6, color=colors, edgecolor="black", linewidth=0.6, yerr=[means[i] - mins[i] for i in range(len(names))], capsize=4)
    for i, (xi, mean, lo, hi) in enumerate(zip(x, means, mins, maxs)):
        jitter = np.full(len(tp[names[i]]), xi) + (np.random.default_rng(0).uniform(-0.15, 0.15, len(tp[names[i]])))
        ax.scatter(jitter, tp[names[i]], s=14, color="black", alpha=0.6, zorder=3)
        ax.errorbar(xi, mean, yerr=[[mean - lo], [hi - mean]], fmt="none", ecolor="black", capsize=4)
    ax.set_xticks(x)
    ax.set_xticklabels(labels)
    ax.set_ylabel("Throughput (Mbps)")
    ax.set_title("Effective throughput comparison")
    for xi, m in zip(x, means):
        ax.annotate(f"{m:.1f}", (xi, m), textcoords="offset points", xytext=(0, 6), ha="center", fontsize=9)
    return _save(fig, "throughput.pdf")


def main() -> int:
    results_dir = sys.argv[1] if len(sys.argv) > 1 else RESULTS_DIR
    if not os.path.isdir(results_dir):
        print(f"results dir not found: {results_dir}")
        return 1
    runs = load_results(results_dir)
    paths = [
        figure1_bee_size(runs),
        figure2_resync_latency(runs),
        figure3_throughput(runs),
    ]
    for p in paths:
        print(f"wrote {p}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
