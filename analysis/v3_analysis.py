#!/usr/bin/env python3
"""Analysis for ChaosSeal V3 experiments (counter baseline, sweeps, corruption test).

Reads results_v3/*.json and produces:
- results_v3/v3_sweep_stats.csv          (R sweep: chaosseal vs counter vs bpsec)
- results_v3/v3_loss_sweep_stats.csv     (loss rate sweep)
- results_v3/v3_size_sweep_stats.csv     (packet-size sweep)
- results_v3/v3_commit_sweep_stats.csv   (commitment interval N sweep)
- results_v3/v3_corruption_stats.csv     (single-bit corruption detection)
- figures in results_v3/figures/
"""
from __future__ import annotations

import glob
import json
import os

import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

RESULTS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "results_v3")
FIGDIR = os.path.join(RESULTS, "figures")
os.makedirs(FIGDIR, exist_ok=True)

MBPS = 1e6
PAYLOAD = 1024


def load_all(results_dir=RESULTS):
    runs = []
    for p in sorted(glob.glob(os.path.join(results_dir, "*.json"))):
        with open(p) as f:
            runs.append(json.load(f))
    return runs


def cs_blk(run, name):
    return run["baselines"].get(name, {}) or {}


def goodput_mbps(run, name):
    """Compute goodput. chaosseal/counter use data_transmission; bpsec uses
    bundle fields directly (payload*8/transfer_sec)."""
    if name == "bpsec":
        dt = cs_blk(run, name)
        return dt["payload_bytes"] * 8 / dt["transfer_sec"] / MBPS
    dt = cs_blk(run, name).get("data_transmission") or {}
    bits = dt["payload_bytes"] * 8
    sec = dt["transfer_sec"] + (dt.get("crypto_wallclock_us", 0) / 1e6)
    return bits / sec / MBPS


def aggregate(rows):
    """Given list of (label, value), return dict label->mean/std/min/max/n."""
    by = {}
    for lab, v in rows:
        by.setdefault(lab, []).append(v)
    out = {}
    for lab, vals in by.items():
        vals = np.array(vals, dtype=float)
        out[lab] = (float(vals.mean()), float(vals.std()), float(vals.min()), float(vals.max()), int(len(vals)))
    return out


def fmt_agg(a):
    return f"{a[0]:.4f}±{a[1]:.4f} (min={a[2]:.4f},max={a[3]:.4f},n={a[4]})"


# ────────────────────────────────────────────────────────────────────────────
# 1. R sweep: chaosseal vs counter vs bpsec
# ────────────────────────────────────────────────────────────────────────────
def r_sweep(runs):
    rows = []
    for r in runs:
        rid = r.get("run_id", "")
        if not rid.startswith("v3-rsweep"):
            continue
        bee_r = r["parameters"]["bee_r"]
        for name in ("chaosseal", "counter", "bpsec"):
            if not cs_blk(r, name):
                continue
            rows.append((f"r{bee_r}.{name}", goodput_mbps(r, name)))
    agg = aggregate((k.split(".")[1] + ":" + k.split(".")[0][1:] for k, _ in []))  # unused
    # structured: dict[name]->dict[r]->agg
    structured = {name: {} for name in ("chaosseal", "counter", "bpsec")}
    for r in runs:
        rid = r.get("run_id", "")
        if not rid.startswith("v3-rsweep"):
            continue
        bee_r = r["parameters"]["bee_r"]
        for name in structured:
            if not cs_blk(r, name):
                continue
            structured[name].setdefault(bee_r, []).append(goodput_mbps(r, name))
    print("=" * 78)
    print("TIER 1a — R-SWEEP GOODPUT (Mbps), N=1024, epoch=1200s, 5 seeds")
    print("=" * 78)
    print(f"{'R':>5} {'chaosseal':>18} {'counter':>18} {'bpsec':>18}")
    for rval in sorted(structured["chaosseal"].keys()):
        line = f"{rval:>5}"
        for name in ("chaosseal", "counter", "bpsec"):
            vals = np.array(structured[name][rval], dtype=float)
            frac = float(rval) / 1024 * 100
            line += f" {vals.mean():>9.4f}±{vals.std():<6.4f}"
        print(line)
    # CSV
    with open(os.path.join(RESULTS, "v3_sweep_stats.csv"), "w") as f:
        f.write("R,chaosseal_mean,chaosseal_std,counter_mean,counter_std,bpsec_mean,bpsec_std\n")
        for rval in sorted(structured["chaosseal"].keys()):
            def m(name):
                v = np.array(structured[name][rval], dtype=float)
                return v.mean(), v.std()
            cm, cs_ = m("chaosseal"); km, ks = m("counter"); bm, bs = m("bpsec")
            f.write(f"{rval},{cm:.6f},{cs_:.6f},{km:.6f},{ks:.6f},{bm:.6f},{bs:.6f}\n")

    # Plot goodput vs R
    fig, ax = plt.subplots(figsize=(8, 5))
    rs = sorted(structured["chaosseal"].keys())
    for name, color, marker in (("chaosseal", "tab:blue", "o"),
                                ("counter", "tab:orange", "s"),
                                ("bpsec", "tab:green", "d")):
        means = [np.mean(structured[name][r]) for r in rs]
        stds = [np.std(structured[name][r]) for r in rs]
        ax.errorbar(rs, means, yerr=stds, label=name.capitalize(), marker=marker, capsize=3)
    ax.set_xscale("log")
    ax.set_xticks(rs)
    ax.set_xticklabels([str(r) for r in rs], rotation=45)
    ax.set_xlabel("Revoked receivers |R| (log)")
    ax.set_ylabel("Goodput (Mbps)")
    ax.set_title("CEP (pendulum) vs counter baseline vs BPsec — goodput vs |R|")
    ax.legend()
    ax.grid(True, which="both", alpha=0.3)
    fig.tight_layout()
    fig.savefig(os.path.join(FIGDIR, "v3_goodput_vs_r.pdf"))
    plt.close(fig)
    return structured


# ────────────────────────────────────────────────────────────────────────────
# 2. Loss sweep
# ────────────────────────────────────────────────────────────────────────────
def loss_sweep(runs):
    rows = []
    for r in runs:
        rid = r.get("run_id", "")
        if not rid.startswith("v3-loss-sweep"):
            continue
        override = r["parameters"].get("loss_rate_override", 0.0) or 0.0
        for name in ("chaosseal", "counter", "bpsec"):
            if not cs_blk(r, name):
                continue
            rows.append((override, name, goodput_mbps(r, name)))
    print()
    print("=" * 78)
    print("TIER 2a — PACKET-LOSS SWEEP (R=8, seed=1): goodput under fixed loss")
    print("=" * 78)
    print(f"{'loss%':>6} {'chaosseal':>16} {'counter':>16} {'bpsec':>16}")
    by = {}
    for ov, name, gp in rows:
        by.setdefault((ov, name), []).append(gp)
    for ov in sorted({k[0] for k in by}):
        line = f"{ov*100:>6.0f}"
        for name in ("chaosseal", "counter", "bpsec"):
            vals = np.array(by.get((ov, name), [0]), dtype=float)
            line += f" {vals.mean():>12.4f}"
        print(line)
    # Also report data-layer resync behaviour
    print("  Data-layer HMAC-resync (per 10,000 packets):")
    for r in runs:
        rid = r.get("run_id", "")
        if not rid.startswith("v3-loss-sweep"):
            continue
        ov = r["parameters"].get("loss_rate_override", 0.0) or 0.0
        for name in ("chaosseal", "counter"):
            ds = r["baselines"].get(name, {}).get("data_stream_loss_sensitivity")
            if ds:
                print(f"    loss={ov*100:.0f}% {name:>10}: resyncs={ds['resyncs']} loss_rate={ds['loss_rate']:.4f}")
    with open(os.path.join(RESULTS, "v3_loss_sweep_stats.csv"), "w") as f:
        f.write("loss_rate,chaosseal_goodput,counter_goodput,bpsec_goodput\n")
        for ov in sorted({k[0] for k in by}):
            g = lambda name: np.mean(by.get((ov, name), [0]))
            f.write(f"{ov},{g('chaosseal'):.6f},{g('counter'):.6f},{g('bpsec'):.6f}\n")


# ────────────────────────────────────────────────────────────────────────────
# 3. Packet-size sweep
# ────────────────────────────────────────────────────────────────────────────
def size_sweep(runs):
    by = {}
    for r in runs:
        rid = r.get("run_id", "")
        if not rid.startswith("v3-size-sweep"):
            continue
        size = r["parameters"]["payload_bytes"]
        for name in ("chaosseal", "counter", "bpsec"):
            if not cs_blk(r, name):
                continue
            by.setdefault((size, name), []).append(goodput_mbps(r, name))
    print()
    print("=" * 78)
    print("TIER 2b — PACKET-SIZE SWEEP (R=8, seed=1): goodput (Mbps)")
    print("=" * 78)
    print(f"{'size B':>8} {'chaosseal':>16} {'counter':>16} {'bpsec':>16}")
    for size in sorted({k[0] for k in by}):
        line = f"{size:>8}"
        for name in ("chaosseal", "counter", "bpsec"):
            vals = np.array(by.get((size, name), [0]), dtype=float)
            line += f" {vals.mean():>12.4f}"
        print(line)
    with open(os.path.join(RESULTS, "v3_size_sweep_stats.csv"), "w") as f:
        f.write("payload_bytes,chaosseal_goodput,counter_goodput,bpsec_goodput\n")
        for size in sorted({k[0] for k in by}):
            g = lambda name: np.mean(by.get((size, name), [0]))
            f.write(f"{size},{g('chaosseal'):.6f},{g('counter'):.6f},{g('bpsec'):.6f}\n")
    # Plot
    fig, ax = plt.subplots(figsize=(8, 5))
    sizes = sorted({k[0] for k in by})
    for name, color, marker in (("chaosseal", "tab:blue", "o"), ("counter", "tab:orange", "s"), ("bpsec", "tab:green", "d")):
        vals = [np.mean(by.get((s, name), [0])) for s in sizes]
        ax.plot(sizes, vals, marker=marker, label=name.capitalize(), color=color)
    ax.set_xscale("log")
    ax.set_xlabel("Payload size (bytes, log)")
    ax.set_ylabel("Goodput (Mbps)")
    ax.set_title("CEP vs counter vs BPsec — goodput vs packet size")
    ax.legend()
    ax.grid(True, which="both", alpha=0.3)
    fig.tight_layout()
    fig.savefig(os.path.join(FIGDIR, "v3_goodput_vs_size.pdf"))
    plt.close(fig)


# ────────────────────────────────────────────────────────────────────────────
# 4. Commitment interval N
# ────────────────────────────────────────────────────────────────────────────
def commit_sweep(runs):
    by = {}
    for r in runs:
        rid = r.get("run_id", "")
        if not rid.startswith("v3-commit-sweep"):
            continue
        n = r["parameters"].get("commit_interval_n")
        dt = cs_blk(r, "chaosseal").get("data_transmission") or {}
        if not dt or n is None:
            continue
        by[n] = goodput_mbps(r, "chaosseal")
    print()
    print("=" * 78)
    print("TIER 2c — COMMITMENT INTERVAL N (HMAC verify every N packets, chaosseal)")
    print("=" * 78)
    print(f"{'N':>6} {'overhead%':>12} {'goodput Mbps':>14}")
    rows = []
    for n in sorted(by):
        rows.append((n, by[n]))
        # find overhead from a run
        for r in runs:
            if r.get("run_id") == f"v3-commit-sweep-n{n}":
                oh = (cs_blk(r, "chaosseal").get("data_transmission") or {})["overhead_pct"] * 100
                break
        print(f"{n:>6} {oh:>12.3f} {by[n]:>14.4f}")
    with open(os.path.join(RESULTS, "v3_commit_sweep_stats.csv"), "w") as f:
        f.write("interval_n,overhead_pct,goodput_mbps\n")
        for n in sorted(by):
            for r in runs:
                if r.get("run_id") == f"v3-commit-sweep-n{n}":
                    oh = (cs_blk(r, "chaosseal").get("data_transmission") or {})["overhead_pct"] * 100
                    break
            f.write(f"{n},{oh:.6f},{by[n]:.6f}\n")
    # Plot
    fig, ax = plt.subplots(figsize=(8, 5))
    ns = sorted(by)
    ax.plot(ns, [by[n] for n in ns], marker="o", color="tab:blue")
    for n in ns:
        for r in runs:
            if r.get("run_id") == f"v3-commit-sweep-n{n}":
                oh = (cs_blk(r, "chaosseal").get("data_transmission") or {})["overhead_pct"] * 100
                break
        ax.annotate(f"{oh:.1f}%", (n, by[n]), textcoords="offset points", xytext=(0, 7), ha="center", fontsize=8)
    ax.set_xscale("log")
    ax.set_xticks(ns)
    ax.set_xticklabels([str(n) for n in ns])
    ax.set_xlabel("Commitment interval N (packets, log)")
    ax.set_ylabel("Goodput (Mbps)")
    ax.set_title("CEP goodput vs HMAC commitment interval N (labels = overhead%)")
    ax.grid(True, which="both", alpha=0.3)
    fig.tight_layout()
    fig.savefig(os.path.join(FIGDIR, "v3_goodput_vs_commit_interval.pdf"))
    plt.close(fig)


# ────────────────────────────────────────────────────────────────────────────
# 5. Corruption test
# ────────────────────────────────────────────────────────────────────────────
def corruption_test(runs):
    for r in runs:
        if r.get("run_id") != "v3-corruption-test":
            continue
        print()
        print("=" * 78)
        print("TIER 1b — SINGLE-BIT CORRUPTION DETECTION (256 bit positions)")
        print("=" * 78)
        s = r["summary"]
        print(f"  Counter mode : mean={s['counter_mean_epochs_to_detect']:.3f} epochs, "
              f"max={s['counter_max_epochs_to_detect']} (immediate bit-flip sensitivity)")
        print(f"  Chaos mode   : mean divergence {s['chaos_mean_divergence_lyap_timescales']:.2f} "
              f"lyap-timescales, max={s['chaos_max_divergence_lyap_timescales']:.2f}")
        print(f"                 key-differs fraction = {s['chaos_key_differs_fraction']*100:.1f}%")
        divs = [p["chaos_divergence_steps"] for p in r["per_bit"]]
        times = [p["chaos_divergence_time_sec"] for p in r["per_bit"]]
        print(f"  Chaos steps  : mean={np.mean(divs):.1f} (≈{np.mean(times):.2f}s), "
              f"min={min(divs)} step (≈{min(times):.3f}s), max={max(divs)} steps (≈{max(times):.2f}s)")
        with open(os.path.join(RESULTS, "v3_corruption_stats.csv"), "w") as f:
            f.write("bit_position,counter_epochs_to_detect,chaos_divergence_steps,chaos_divergence_time_sec,chaos_lyapunov_timescales,key_differs\n")
            for p in r["per_bit"]:
                f.write(f"{p['bit_position']},{p['counter_epochs_until_detect']},"
                        f"{p['chaos_divergence_steps']},{p['chaos_divergence_time_sec']:.6f},"
                        f"{p['chaos_lyapunov_timescales']:.6f},{p['chaos_key_differs']}\n")
        # Plot divergence-time distribution per bit
        fig, ax = plt.subplots(figsize=(8, 5))
        ax.hist(times, bins=30, color="tab:blue", alpha=0.7)
        ax.axvline(np.mean(times), color="red", linestyle="--", label=f"mean={np.mean(times):.1f}s")
        ax.set_xlabel("Chaos-mode divergence time (s) for a single-bit corruption")
        ax.set_ylabel("Bit positions (count)")
        ax.set_title("Pendulum 1-bit perturbation → O(1) state divergence time")
        ax.legend()
        ax.grid(True, alpha=0.3)
        fig.tight_layout()
        fig.savefig(os.path.join(FIGDIR, "v3_corruption_divergence.pdf"))
        plt.close(fig)
        break


def main():
    runs = load_all()
    r_sweep(runs)
    loss_sweep(runs)
    size_sweep(runs)
    commit_sweep(runs)
    corruption_test(runs)
    print("\nAll analyses complete. Outputs in", RESULTS)


if __name__ == "__main__":
    main()
