#!/usr/bin/env python3
"""Commitment-interval sensitivity under realistic LEO loss profiles.

Models the interaction between HMAC commitment interval (verify every N packets)
and Gilbert-Elliott burst loss. The key effect: during a burst, packets are lost
including HMAC verification packets, which delays desync detection.

Uses existing v3_commit_sweep_stats.csv and v3_loss_sweep_stats.csv as baselines,
and computes a theoretical model of the combined effect.
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

# Default Gilbert-Elliott parameters from the simulation
DEFAULT_LOSS = {
    "PGoodToBad": 0.05,
    "PBadToGood": 0.35,
    "GoodLoss": 0.001,
    "BadLoss": 0.25,
}

# Loss profiles to test (matching v3_loss_sweep plus burst variants)
LOSS_PROFILES = {
    "clean":     {"PGoodToBad": 0.0,   "PBadToGood": 1.0, "GoodLoss": 0.0,   "BadLoss": 0.0},
    "light":     {"PGoodToBad": 0.01,  "PBadToGood": 0.5, "GoodLoss": 0.001, "BadLoss": 0.1},
    "default":   DEFAULT_LOSS,
    "heavy":     {"PGoodToBad": 0.1,   "PBadToGood": 0.2, "GoodLoss": 0.01,  "BadLoss": 0.5},
    "extreme":   {"PGoodToBad": 0.2,   "PBadToGood": 0.1, "GoodLoss": 0.05,  "BadLoss": 0.8},
}

# Stationary distribution of Gilbert-Elliott
def stationary_loss_rate(pgb, pbg, gl, bl):
    """Compute steady-state loss rate."""
    # pi_good = pbg / (pgb + pbg)
    # pi_bad = pgb / (pgb + pbg)
    # loss_rate = pi_good * gl + pi_bad * bl
    pi_bad = pgb / (pgb + pbg) if (pgb + pbg) > 0 else 0
    pi_good = 1 - pi_bad
    return pi_good * gl + pi_bad * bl

def burst_length_stats(pbg, bl):
    """Expected burst length and loss within burst."""
    # Geometric distribution with success prob = pbg
    exp_burst_len = 1.0 / pbg if pbg > 0 else float('inf')
    # Expected losses in a burst = burst_len * bl
    exp_burst_losses = exp_burst_len * bl
    return exp_burst_len, exp_burst_losses

def commitment_miss_prob(N, pgb, pbg, gl, bl, packets_per_epoch):
    """Probability that a commitment verification packet is lost due to burst.
    
    The effective verification interval under loss is N / (1 - loss_rate).
    But more precisely: if verification packets are sent every N packets,
    and we lose a fraction 'loss_rate' of packets, then the expected number
    of sent packets between *received* verifications is N / (1 - loss_rate).
    
    Under burst loss, bursts can cause multiple consecutive missed verifications.
    """
    loss_rate = stationary_loss_rate(pgb, pbg, gl, bl)
    # Expected verifications per epoch
    verifications_per_epoch = packets_per_epoch / N
    # Expected received verifications
    received_verifications = verifications_per_epoch * (1 - loss_rate)
    # Expected gap between received verifications (in packets)
    if received_verifications > 0:
        effective_interval = packets_per_epoch / received_verifications
    else:
        effective_interval = float('inf')
    return effective_interval, loss_rate

def model_goodput_loss(base_goodput, N, loss_config, packets_per_epoch=1000):
    """Model goodput degradation due to commitment interval under loss.
    
    Key effects:
    1. Lost HMAC verification packets delay desync detection
    2. During burst, multiple verifications missed -> longer resync time
    3. Overhead from HMAC is 32 bytes every N packets = 32/N per packet
    
    The base_goodput already includes HMAC overhead. We model additional
    loss-induced desync/recovery overhead.
    """
    pgb = loss_config["PGoodToBad"]
    pbg = loss_config["PBadToGood"]
    gl = loss_config["GoodLoss"]
    bl = loss_config["BadLoss"]
    
    loss_rate = stationary_loss_rate(pgb, pbg, gl, bl)
    burst_len, burst_losses = burst_length_stats(pbg, bl)
    
    # Effective verification interval accounting for loss
    effective_N, _ = commitment_miss_prob(N, pgb, pbg, gl, bl, packets_per_epoch)
    
    # HMAC overhead already in base_goodput
    # Additional overhead: desync detection delay
    # If desync occurs, it takes up to effective_N packets to detect
    # Each epoch has packets_per_epoch / N verification points
    # Expected desync events per epoch: depends on loss and drift
    # Model: desync probability proportional to loss_rate * effective_N / packets_per_epoch
    # Recovery cost: re-sync latency (estimated ~50ms from v3 data)
    
    # Simple model: additional latency per epoch = loss_rate * effective_N * resync_penalty
    resync_penalty_ms = 50  # from v3 resync latency data
    additional_latency_ms = loss_rate * (effective_N / packets_per_epoch) * resync_penalty_ms * 1000
    
    # Goodput reduction
    overhead_factor = 1.0 + additional_latency_ms / 1000.0 / 1200.0  # per epoch
    
    return base_goodput / overhead_factor, effective_N, loss_rate, burst_len

def load_commit_sweep():
    """Load base commit sweep data (no loss)."""
    path = RESULTS / "v3_commit_sweep_stats.csv"
    data = {}
    with open(path) as f:
        r = csv.DictReader(f)
        for row in r:
            N = int(row["interval_n"])
            data[N] = float(row["goodput_mbps"])
    return data

def main():
    print("=" * 70)
    print("Commitment-Interval Sensitivity Under LEO Loss Profiles")
    print("=" * 70)
    
    base_goodputs = load_commit_sweep()
    print(f"Base commit sweep (no loss): {base_goodputs}")
    print()
    
    # Parameters
    packets_per_epoch = 1200 * 1.65e6 / 1024 / 8  # estimated from 1.65 Mbps goodput
    print(f"Estimated packets per 1200s epoch: {packets_per_epoch:.0f}")
    
    results = {}
    
    for profile_name, loss_config in LOSS_PROFILES.items():
        print(f"\nProfile: {profile_name}")
        print(f"  Params: {loss_config}")
        
        loss_rate = stationary_loss_rate(
            loss_config["PGoodToBad"], loss_config["PBadToGood"],
            loss_config["GoodLoss"], loss_config["BadLoss"]
        )
        burst_len, burst_losses = burst_length_stats(
            loss_config["PBadToGood"], loss_config["BadLoss"]
        )
        print(f"  Stationary loss rate: {loss_rate:.4f}")
        print(f"  Expected burst length: {burst_len:.1f} packets")
        print(f"  Expected losses per burst: {burst_losses:.1f}")
        
        profile_results = {}
        for N, base_gp in base_goodputs.items():
            gp, eff_N, lr, bl = model_goodput_loss(base_gp, N, loss_config, packets_per_epoch)
            profile_results[N] = {
                "goodput_mbps": gp,
                "effective_N": eff_N,
                "loss_rate": lr,
                "burst_len": bl,
            }
            print(f"  N={N:>4}: base={base_gp:.4f} Mbps -> loss={gp:.4f} Mbps (eff_N={eff_N:.1f})")
        
        results[profile_name] = profile_results
    
    # Save combined CSV
    out_csv = RESULTS / "v3_commit_loss_sweep_stats.csv"
    with open(out_csv, 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(["profile", "commit_interval_N", "base_goodput_mbps", "loss_goodput_mbps",
                    "effective_N", "stationary_loss_rate", "burst_len"])
        for profile, pres in results.items():
            loss_config = LOSS_PROFILES[profile]
            lr = stationary_loss_rate(loss_config["PGoodToBad"], loss_config["PBadToGood"],
                                      loss_config["GoodLoss"], loss_config["BadLoss"])
            bl, _ = burst_length_stats(loss_config["PBadToGood"], loss_config["BadLoss"])
            for N, d in pres.items():
                w.writerow([profile, N, base_goodputs[N], d["goodput_mbps"],
                            d["effective_N"], lr, bl])
    print(f"\nSaved combined stats to {out_csv}")
    
    # Plot 1: Goodput vs N for each loss profile
    fig, ax = plt.subplots(figsize=(10, 6))
    colors = {"clean": "tab:green", "light": "tab:blue", "default": "tab:orange",
              "heavy": "tab:red", "extreme": "tab:purple"}
    markers = {"clean": "o", "light": "s", "default": "d", "heavy": "^", "extreme": "v"}
    
    Ns = sorted(base_goodputs.keys())
    for profile in ["clean", "light", "default", "heavy", "extreme"]:
        pres = results[profile]
        gps = [pres[N]["goodput_mbps"] for N in Ns]
        lc = LOSS_PROFILES[profile]
        lr = stationary_loss_rate(lc["PGoodToBad"], lc["PBadToGood"], lc["GoodLoss"], lc["BadLoss"])
        ax.plot(Ns, gps, marker=markers[profile], color=colors[profile],
                label=f"{profile} (loss={lr:.3f})")
    
    ax.set_xscale("log")
    ax.set_xticks(Ns)
    ax.set_xticklabels([str(n) for n in Ns])
    ax.set_xlabel("Commitment interval N (packets)")
    ax.set_ylabel("Goodput (Mbps)")
    ax.set_title("CEP Goodput vs HMAC Commitment Interval\nUnder Different LEO Loss Profiles")
    ax.legend(fontsize=8)
    ax.grid(True, which="both", alpha=0.3)
    fig.tight_layout()
    fig.savefig(FIG / "v3_goodput_vs_commit_interval_loss.pdf")
    plt.close(fig)
    print(f"Saved {FIG / 'v3_goodput_vs_commit_interval_loss.pdf'}")
    
    # Plot 2: Relative degradation vs loss rate
    fig, ax = plt.subplots(figsize=(10, 6))
    for profile in ["light", "default", "heavy", "extreme"]:
        pres = results[profile]
        loss_config = LOSS_PROFILES[profile]
        lr = stationary_loss_rate(loss_config["PGoodToBad"], loss_config["PBadToGood"],
                                  loss_config["GoodLoss"], loss_config["BadLoss"])
        degradation = [(pres[N]["goodput_mbps"] / base_goodputs[N] - 1) * 100 for N in Ns]
        ax.plot(Ns, degradation, marker=markers[profile], color=colors[profile],
                label=f"{profile} (loss={lr:.3f})")
    
    ax.axhline(0, color="k", lw=0.5)
    ax.set_xscale("log")
    ax.set_xticks(Ns)
    ax.set_xticklabels([str(n) for n in Ns])
    ax.set_xlabel("Commitment interval N (packets)")
    ax.set_ylabel("Goodput degradation vs no-loss (%)")
    ax.set_title("Relative Goodput Degradation Due to Burst Loss\n(Lower N = more frequent HMAC = faster desync detection)")
    ax.legend(fontsize=8)
    ax.grid(True, which="both", alpha=0.3)
    fig.tight_layout()
    fig.savefig(FIG / "v3_commit_loss_degradation.pdf")
    plt.close(fig)
    print(f"Saved {FIG / 'v3_commit_loss_degradation.pdf'}")
    
    # Plot 3: Effective verification interval
    fig, ax = plt.subplots(figsize=(10, 6))
    for profile in ["light", "default", "heavy", "extreme"]:
        pres = results[profile]
        eff_Ns = [pres[N]["effective_N"] for N in Ns]
        ax.plot(Ns, eff_Ns, marker=markers[profile], color=colors[profile], label=profile)
    
    # Ideal line (no loss)
    ax.plot(Ns, Ns, 'k--', alpha=0.5, label="No loss (ideal)")
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xticks(Ns)
    ax.set_xticklabels([str(n) for n in Ns])
    ax.set_xlabel("Configured commitment interval N (packets)")
    ax.set_ylabel("Effective interval between *received* verifications")
    ax.set_title("Burst Loss Inflates Effective HMAC Verification Interval")
    ax.legend(fontsize=8)
    ax.grid(True, which="both", alpha=0.3)
    fig.tight_layout()
    fig.savefig(FIG / "v3_effective_commit_interval.pdf")
    plt.close(fig)
    print(f"Saved {FIG / 'v3_effective_commit_interval.pdf'}")
    
    # Summary table
    print("\n" + "=" * 70)
    print("SUMMARY: Goodput (Mbps) at key commitment intervals")
    print("=" * 70)
    key_Ns = [1, 4, 16, 64, 256, 1024]
    print(f"{'Profile':>10} " + " ".join(f"N={n:<4}" for n in key_Ns))
    for profile in ["clean", "light", "default", "heavy", "extreme"]:
        pres = results[profile]
        vals = [f"{pres.get(n, {}).get('goodput_mbps', 0):.4f}" for n in key_Ns]
        print(f"{profile:>10} " + " ".join(f"{v:>8}" for v in vals))
    
    print("\nDone.")

if __name__ == "__main__":
    main()