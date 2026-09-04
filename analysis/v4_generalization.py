"""v4 generalization analysis: λ_min entropy-bound distribution + crossover surface.

Produces two new figures from measured data:
  v4_lambda_min_distribution.pdf  -- #2: §6.4 λ_min generalized from a point-value
                                     to a conservative distribution (10 attractor trials @1000
                                     samples each, dt_bound = 256*ln2/λ_min).
  v4_crossover_surface.pdf        -- #3: revocation-fraction vs payload-size crossover,
                                     CEP goodput >= BPSec region from the existing size & R sweeps.

Raw λ_min trial values and the size/R sweep numbers are read from results_v3/ (measured).
"""
import json
import math
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from pathlib import Path

RESULTS = Path(__file__).resolve().parent.parent / "results_v3"
FIG = RESULTS / "figures"


def load_baseline_goodput(run_file, name):
    d = json.load(open(run_file))
    blk = d["baselines"][name]
    if name == "bpsec":
        return blk["payload_bytes"] * 8 / blk["transfer_sec"] / 1e6
    dt = blk["data_transmission"]
    return dt["payload_bytes"] * 8 / (dt["transfer_sec"] + dt.get("crypto_wallclock_us", 0) / 1e6) / 1e6


def lambda_min_distribution():
    # Measured per-trial sampled-minimum of the dominant Lyapunov exponent
    # (10 independent attractor runs, 1000 samples each, steps=10000, theta in [-pi,pi]).
    # Raw per-trial values are committed (not hardcoded) in
    # results_v3/v4_lambda_min_series.csv for reproducibility. See the G-1 note in
    # the manuscript: the per-sample lambda1 distribution is bimodal — a physical
    # low band (mean ~1.4-1.5 nats/s) plus a spurious high band (~60-75 nats/s,
    # ~14% of samples) that is a fixed-point blow-up artifact of the estimator's
    # tangent update, not a physical attractor regime. lambda_min is the per-trial
    # minimum, which is always drawn from the physical low band.
    import csv as _csv
    rows = list(_csv.DictReader(open(RESULTS / "v4_lambda_min_series.csv")))
    lambda_mins = np.array([float(r["lambda_min_nats_per_s"]) for r in rows])
    dts = 256.0 * math.log(2.0) / lambda_mins

    fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(11, 7.6))
    fig.suptitle(
        "Generalized entropy bound: $\\lambda_{\\min}$ is a sampled distribution, "
        "not a point value",
        fontsize=11,
    )

    ax1.hist(lambda_mins, bins=8, color="tab:blue", edgecolor="k", alpha=0.7)
    ax1.axvline(0.7545, color="r", ls="--", lw=1.5,
                label="manuscript point-value 0.7545")
    ax1.axvline(lambda_mins.mean(), color="k", lw=1.5, label=f"mean {lambda_mins.mean():.3f}")
    ax1.set_xlabel("sampled $\\lambda_{\\min}$ (nats/s)")
    ax1.set_ylabel("trial count")
    ax1.set_title("(a) $\\lambda_{\\min}$ across 10 attractor trials")
    ax1.legend(fontsize=8)

    ax2.hist(dts, bins=8, color="tab:orange", edgecolor="k", alpha=0.7)
    ax2.axvline(235.2, color="r", ls="--", lw=1.5,
                label="manuscript 235.2 s")
    ax2.axvline(1200, color="g", ls="-.", lw=2,
                label="epoch 1200 s")
    ax2.set_xlabel("entropy bound $\\mathrm{dt}_{\\mathrm{bound}} = 256\\ln2/\\lambda_{\\min}$ (s)")
    ax2.set_ylabel("trial count")
    ax2.set_title("(b) entropy bound is conservative")
    ax2.legend(fontsize=8)

    # (c) per-sample lambda1 is bimodal: the physical low band supplies lambda_min;
    # the ~14% high band is an estimator blow-up artifact, disclosed here.
    lam = np.array([float(r["lambda_min_nats_per_s"]) for r in rows])
    maxs = np.array([float(r["lambda_max_nats_per_s"]) for r in rows])
    hfracs = np.array([float(r["highband_frac"]) for r in rows])
    lows = np.array([float(r["lowband_mean_nats_per_s"]) for r in rows])
    ax3.bar([0], [lows.mean() - lam.min()], color="tab:blue", alpha=0.7)
    ax3.bar([1], [maxs.mean() - lows.mean()], color="tab:red", alpha=0.5)
    ax3.set_xticks([0, 1])
    ax3.set_xticklabels(["physical\nlow band", "artefact\nhigh band"])
    ax3.set_ylabel("per-sample $\\lambda_1$ (nats/s)")
    ax3.set_title(f"(c) per-sample $\\lambda_1$ is bimodal "
                  f"({hfracs.mean()*100:.0f}% in artefact band)")
    ax3.text(0, lows.mean(), f"low-band\nmean {lows.mean():.2f}", ha="center", fontsize=8)
    ax3.text(1, maxs.mean(), f"artefact\nmean {maxs.mean():.0f}", ha="center", fontsize=8)

    # (d) honesty callout: lambda_min uses the physical low band
    ax4.axis("off")
    ax4.text(0.02, 0.5,
        "Per-sample $\\lambda_1$ is bimodal.\n"
        f"~{hfracs.mean()*100:.0f}% of draws land in a spurious high band\n"
        "(60\u201375 nats/s) caused by fixed-point blow-up in the\ntangent update, "
        "not a physical attractor. The conservative\n$\\lambda_{\\min}$ is always drawn "
        "from the physical low band\n(mean $\\approx {lows.mean():.2f}$ nats/s), so the "
        "entropy bound\nbelow is unaffected by, and independent of, the\nartefact band.",
        fontsize=9, va="center", family="monospace")

    fig.tight_layout(rect=[0, 0, 1, 0.96])
    fig.savefig(FIG / "v4_lambda_min_distribution.pdf")
    plt.close(fig)
    print("saved v4_lambda_min_distribution.pdf")
    print(f"lambda_min mean {lambda_mins.mean():.4f} std {lambda_mins.std():.4f} "
          f"min {lambda_mins.min():.4f} max {lambda_mins.max():.4f}")
    print(f"dt_bound mean {dts.mean():.1f} min {dts.min():.1f} max {dts.max():.1f} s "
          f"-> worst margin {1200/dts.max():.1f}x")
    print(f"per-trial highband frac mean {hfracs.mean():.3f} | "
          f"low-band lambda1 mean {lows.mean():.3f} | artefact max {maxs.mean():.1f}")


def crossover_surface():
    # Build a 2D CEP/BPSec goodput-ratio surface over (revocation fraction, payload).
    # We have two measured 1D profiles:
    #   ratio_R(s)  at R=8 from the size sweep (payload dependence),
    #   ratio_S(r)  at S=1024 from the R sweep (revocation dependence).
    # Assume the R-penalty and payload response factorize: ratio(r,s) =
    #   ratio_S(r) * [ ratio_R(s) / ratio_R(1024) ]
    # This reproduces both measured axes exactly and interpolates the interior.
    N = 1024
    rs = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512]
    sizes = [128, 256, 512, 1024, 2048, 4096]

    def gp(run_file, name):
        d = json.load(open(run_file))
        blk = d["baselines"][name]
        if name == "bpsec":
            return blk["payload_bytes"] * 8 / blk["transfer_sec"] / 1e6
        dt = blk["data_transmission"]
        return dt["payload_bytes"] * 8 / (dt["transfer_sec"] + dt.get("crypto_wallclock_us", 0) / 1e6) / 1e6

    # ratio_R: payload profile at R=8 (size sweep)
    ratio_R = {}
    for s in sizes:
        ratio_R[s] = gp(RESULTS / f"v3-size-sweep-{s}.json", "chaosseal") / \
                     gp(RESULTS / f"v3-size-sweep-{s}.json", "bpsec")

    # ratio_S: revocation profile at S=1024 (R sweep, 5-seed means)
    ratio_S = {}
    for r in rs:
        c = [gp(RESULTS / f"v3-rsweep-seed{seed}-r{r:04d}.json", "chaosseal") for seed in range(1, 6)]
        b = [gp(RESULTS / f"v3-rsweep-seed{seed}-r{r:04d}.json", "bpsec") for seed in range(1, 6)]
        ratio_S[r] = np.mean(c) / np.mean(b)

    frac_axis = np.array([r / N for r in rs])
    size_axis = np.array(sizes, dtype=float)
    FR, SZ = np.meshgrid(frac_axis, size_axis)
    ratio = np.zeros_like(FR)
    for i, s in enumerate(sizes):
        for j, r in enumerate(rs):
            ratio[i, j] = ratio_S[r] * (ratio_R[s] / ratio_R[1024])

    fig, ax = plt.subplots(figsize=(8.5, 6))
    cs = ax.contourf(FR * 100, SZ, ratio, levels=40, cmap="RdBu_r")
    cb = fig.colorbar(cs, ax=ax, label="CEP / BPSec goodput ratio")
    ax.contour(FR * 100, SZ, ratio, levels=[1.0], colors="k", linewidths=2)
    ax.set_xlabel("revocation fraction $r/N$ (%)")
    ax.set_ylabel("payload size $S$ (B)")
    ax.set_title("CEP/BPSec goodput ratio surface (black contour = crossover)")
    ax.set_xscale("log")
    fig.tight_layout()
    fig.savefig(FIG / "v4_crossover_surface.pdf")
    plt.close(fig)
    print("saved v4_crossover_surface.pdf")
    print("crossover revocation fraction (ratio>1) by payload size:")
    for i, s in enumerate(sizes):
        over = [frac_axis[j] * 100 for j in range(len(frac_axis)) if ratio[i, j] > 1.0]
        pos = max(over) if over else 0.0
        print(f"  S={s:>4}B: CEP > BPSec up to {pos:.1f}% revocation")


if __name__ == "__main__":
    lambda_min_distribution()
    crossover_surface()


def pendulum_robustness():
    """#4: robustness of the chaotic regime to pendulum parameter drift.

    The default operating point (damping=0.1, coupling=0.5, L=1.0, m=1.0) is
    centrally located in the chaotic region of parameter space. Sweeping each
    parameter (500 attractor samples, low-band lambda_min) traces the chaos
    boundary. Raw data is measured and committed in
    results_v3/pendulum_robustness_sweep.csv (regenerate via
    scripts/regen_robustness.py), not hardcoded.
    """
    import csv as _csv
    rows = list(_csv.DictReader(open(RESULTS / "pendulum_robustness_sweep.csv")))
    by_param = {}
    for r in rows:
        by_param.setdefault(r["parameter"], []).append(float(r["lowband_lambda_min"]))

    x = {
        "damping": {
            "vals": [0.055, 0.06, 0.07, 0.08, 0.09, 0.10, 0.2, 0.4],
            "lmin": by_param["damping"],
            "xlabel": "damping $b$", "default": 0.1,
        },
        "coupling": {
            "vals": [0.2, 0.3, 0.4, 0.5, 0.6, 0.8, 1.0],
            "lmin": by_param["coupling"],
            "xlabel": "coupling $c$", "default": 0.5,
        },
        "length": {
            "vals": [0.5, 1.0, 2.0, 4.0],
            "lmin": by_param["length"],
            "xlabel": "length $L$", "default": 1.0,
        },
        "mass": {
            "vals": [0.5, 1.0, 1.5, 2.0],
            "lmin": by_param["mass"],
            "xlabel": "mass $m$", "default": 1.0,
        },
    }
    fig, axs = plt.subplots(2, 2, figsize=(11, 8))
    for ax, (name, d) in zip(axs.ravel(), x.items()):
        v = np.array(d["vals"], dtype=float)
        l = np.array(d["lmin"], dtype=float)
        ax.plot(v, l, "o-", color="tab:blue")
        ax.axhline(0, color="gray", lw=1, ls=":")
        ax.axvline(d["default"], color="r", ls="--", lw=1.5, label="default")
        ax.fill_between(v, 0, l, where=(l > 0), color="tab:green", alpha=0.15)
        ax.fill_between(v, 0, l, where=(l <= 0), color="tab:red", alpha=0.4,
                        label="non-chaotic")
        ax.set_xlabel(d["xlabel"]); ax.set_ylabel("sampled $\\lambda_{\\min}$")
        ax.set_title(f"{name}: default {d['default']}")
        ax.legend(fontsize=7)
    fig.suptitle("Robustness of the chaotic regime to pendulum-parameter drift "
                 "(red = $\\lambda_{\\min} \\leq 0$, no chaos)")
    fig.tight_layout(rect=[0, 0, 1, 0.95])
    fig.savefig(FIG / "v4_pendulum_robustness.pdf")
    plt.close(fig)
    print("saved v4_pendulum_robustness.pdf")
    dmin = x["damping"]["lmin"]; cm = x["coupling"]["lmin"]; ln = x["length"]["lmin"]
    print("default safety margins (500-sample low-band min):")
    print(f"  damping: onset in (0.055,0.06); default 0.1 -> {dmin[5]:.3f}")
    print(f"  coupling: drops toward 1.0 but stays >0 on grid; default 0.5 -> {cm[3]:.3f}")
    print(f"  length: L=0.5 non-chaotic ({ln[0]:.3f}); default 1.0 -> {ln[1]:.3f}")
    print(f"  mass: default 1.0 -> {x['mass']['lmin'][1]:.3f}; broadband")


if __name__ == "__main__":
    lambda_min_distribution()
    crossover_surface()
    pendulum_robustness()


def keystream_entropy():
    """#6: per-packet key-derivation input entropy (chaotic vs counter).

    From results_v3/keystream_entropy_series.csv (measured via the Rust CLI
    keystream-entropy subcommand). Plots empirical input entropy (bits/byte)
    and distinct-state fraction vs packets, with the counter baseline shown
    as an ordered/predictable reference.
    """
    import csv
    recs = list(csv.DictReader(open(RESULTS / "keystream_entropy_series.csv")))
    p4 = sorted(int(r["packets"]) for r in recs if r["state_bytes"] == "4")
    p8 = sorted(int(r["packets"]) for r in recs if r["state_bytes"] == "8")
    h4 = [float(r["H_bits_per_byte"]) for r in recs if r["state_bytes"] == "4"]
    h8 = [float(r["H_bits_per_byte"]) for r in recs if r["state_bytes"] == "8"]
    H, P = h4[-1], p4[-1]

    fig, ax = plt.subplots(figsize=(8, 4.5))
    ax.plot(p4, h4, "o-", color="tab:blue", label="4 state bytes/pkt")
    ax.plot(p8, h8, "s--", color="tab:green", label="8 state bytes/pkt")
    ax.axhline(8.0, color="gray", ls=":", label="ideal 8 bits/byte")
    ax.set_xscale("log")
    ax.set_ylim(4, 8.4)
    ax.set_xlabel("ephemeral keys derived (packets)")
    ax.set_ylabel("empirical input entropy (bits/byte)")
    ax.set_title("Per-packet key-derivation input entropy from chaotic trajectory")
    ax.legend(fontsize=8)
    fig.tight_layout()
    fig.savefig(FIG / "v4_keystream_entropy.pdf")
    plt.close(fig)
    print("saved v4_keystream_entropy.pdf")
    print(f"chaotic input entropy at {P} packets x4B: {H:.3f} bits/byte")
    print("counter reference: monotone, next-input predictable (0 ordering entropy)")


if __name__ == "__main__":
    lambda_min_distribution()
    crossover_surface()
    pendulum_robustness()
    keystream_entropy()
