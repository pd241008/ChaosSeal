#!/usr/bin/env python3
"""Print the actual numbers (mean / median / p95) quoted in the paper.

Reads /results/*.json produced by netsim. Never recomputes protocol or
simulation logic; every number printed here traces back to a JSON field.

Usage:
    python3 stats.py [results_dir]     # default: ../results relative to this file
"""
from __future__ import annotations

import glob
import json
import os
import sys

import numpy as np

RESULTS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "results")

MBPS = 1e6  # bits per second


def load_results(results_dir: str = RESULTS_DIR) -> list[dict]:
    """Load every result JSON under results_dir, sorted by run_id."""
    runs = []
    for path in sorted(glob.glob(os.path.join(results_dir, "*.json"))):
        with open(path) as f:
            runs.append(json.load(f))
    return runs


def main_runs(runs: list[dict]) -> list[dict]:
    """Runs matching the paper's parameter set (N=1024, R=8) with all three
    baselines exercised."""
    return [
        r
        for r in runs
        if r.get("baselines")
        and "tls13" in r["baselines"]
        and r.get("parameters", {}).get("bee_r") == 8
    ]


def bee_sizes(runs: list[dict]) -> dict[int, list[int]]:
    """Map |R| -> list of BEE ciphertext sizes (bytes) across runs."""
    by_r: dict[int, list[int]] = {}
    for r in runs:
        bee = r.get("baselines", {}).get("chaosseal", {}).get("bee")
        if not bee:
            continue
        by_r.setdefault(bee["r"], []).append(bee["ciphertext_size_bytes"])
    return by_r


def resync_latency_ms(runs: list[dict]) -> list[float]:
    """Propagation latency of every BEE revocation update across runs."""
    out: list[float] = []
    for r in runs:
        msgs = r.get("baselines", {}).get("chaosseal", {}).get("revocation_messages", [])
        out.extend(m["latency_ms"] for m in msgs if "latency_ms" in m)
    return out


def link_latency_samples_ms(runs: list[dict]) -> list[float]:
    """One-way latency of every visible satellite link sample across runs."""
    out: list[float] = []
    for r in runs:
        out.extend(r.get("link_stats", {}).get("latency_samples_ms", []))
    return out


def throughput_mbps(runs: list[dict]) -> dict[str, list[float]]:
    """Effective throughput per baseline: bits moved / total operation time.

    ChaosSeal:  bits = sum(ciphertext_bytes*8 over R updates)
                time = sum(transfer_sec + latency_ms/1000 over R updates)
    TLS 1.3:    bits = (bytes_sent + bytes_received) * 8
                time = handshake_sec + app_payload_sec
    BPSec:      bits = bundle_size_bytes * 8
                time = transfer_sec   (already includes propagation latency)
    """
    out: dict[str, list[float]] = {"chaosseal": [], "tls13": [], "bpsec": []}
    for r in runs:
        bs = r.get("baselines", {})

        cs = bs.get("chaosseal")
        if cs and cs.get("revocation_messages"):
            msgs = cs["revocation_messages"]
            bits = sum(m["ciphertext_bytes"] for m in msgs) * 8
            sec = sum(m["transfer_sec"] + m["latency_ms"] / 1000 for m in msgs)
            if sec > 0:
                out["chaosseal"].append(bits / sec / MBPS)

        tls = bs.get("tls13")
        if tls and tls.get("handshake_sec") is not None:
            bits = (tls.get("bytes_sent", 0) + tls.get("bytes_received", 0)) * 8
            sec = tls["handshake_sec"] + tls.get("app_payload_sec", 0)
            if sec > 0:
                out["tls13"].append(bits / sec / MBPS)

        bp = bs.get("bpsec")
        if bp and bp.get("bundle_size_bytes"):
            sec = bp.get("transfer_sec", 0)
            if sec > 0:
                out["bpsec"].append(bp["bundle_size_bytes"] * 8 / sec / MBPS)
    return out


def _pct(values: list[float], p: float) -> float:
    if not values:
        return float("nan")
    return float(np.percentile(values, p))


def fmt_stats(values: list[float]) -> str:
    if not values:
        return "n/a"
    return f"mean={np.mean(values):.4g} median={_pct(values, 50):.4g} p95={_pct(values, 95):.4g} (n={len(values)})"


def _fmt_row(label: str, value) -> str:
    return f"  {label:<28} {value}"


def print_stats(runs: list[dict]) -> None:
    mains = main_runs(runs)
    if not mains:
        print("no runs with all baselines found in results dir")
        return

    seeds = [r["rng_seed"] for r in mains]
    print(f"main runs: {len(mains)} (seeds {sorted(seeds)})")
    print()

    print("=== Link (main runs) ===")
    visible = [r["link_stats"]["visible_pct"] for r in mains]
    latency = [r["link_stats"]["mean_latency_ms"] for r in mains]
    loss = [r["link_stats"]["loss_rate"] for r in mains]
    print(_fmt_row("visible_pct", f"{np.mean(visible):.2f}% (n={len(visible)})"))
    print(_fmt_row("mean_latency_ms", fmt_stats(latency)))
    print(_fmt_row("loss_rate", fmt_stats(loss)))
    print()

    print("=== Lyapunov (chaosseal baseline) ===")
    lambdas = [r["baselines"]["chaosseal"]["lyapunov"]["lambda1"] for r in mains]
    dt = [r["baselines"]["chaosseal"]["lyapunov"]["dt_bound"] for r in mains]
    print(_fmt_row("lambda1", fmt_stats(lambdas)))
    print(_fmt_row("dt_bound", fmt_stats(dt)))
    print()

    print("=== BEE (N=1024, R=8) ===")
    sizes = [r["baselines"]["chaosseal"]["bee"]["ciphertext_size_bytes"] for r in mains]
    print(_fmt_row("ciphertext_size_bytes", fmt_stats(sizes)))
    print()

    print("=== Resynchronization latency ===")
    rev = resync_latency_ms(mains)
    print(_fmt_row("revocation link latency_ms", fmt_stats(rev)))
    links = link_latency_samples_ms(mains)
    print(_fmt_row("visible link latency_ms (samples)", fmt_stats(links)))
    print()

    print("=== Throughput (Mbps) ===")
    for name, vals in throughput_mbps(mains).items():
        print(_fmt_row(name, fmt_stats(vals)))
    print()

    print("=== Delivery (chaosseal) ===")
    delivered = [r["baselines"]["chaosseal"]["delivery"]["delivered"] for r in mains]
    sent = [r["baselines"]["chaosseal"]["delivery"]["sent"] for r in mains]
    print(_fmt_row("delivered/sent", f"{sum(delivered)}/{sum(sent)}"))
    print(_fmt_row("loss_rate", f"{np.mean([r['baselines']['chaosseal']['delivery']['loss_rate'] for r in mains]):.4g}"))


def main() -> int:
    results_dir = sys.argv[1] if len(sys.argv) > 1 else RESULTS_DIR
    if not os.path.isdir(results_dir):
        print(f"results dir not found: {results_dir}")
        return 1
    print_stats(load_results(results_dir))
    return 0


if __name__ == "__main__":
    sys.exit(main())
