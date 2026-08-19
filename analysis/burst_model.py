#!/usr/bin/env python3
import os
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

# Parameters for R=512
R = 512
N = 1024
BEE_CIPHERTEXT = 65536
HMAC_LEN = 16
PAYLOAD = 1024
DOWNLINK_BPS = 50e6
LATENCY_BEST = 0.00485 # 4.85 ms average best-measurement latency across the 600s window sweeps

def main():
    # Burst phase (R update messages, pipelined without individual latency overhead to match simulation area)
    t_burst = R * (BEE_CIPHERTEXT * 8 / DOWNLINK_BPS)
    
    # Steady phase (N payload messages, stop-and-wait simulated with one-way latency)
    t_steady_per_msg = ((PAYLOAD + HMAC_LEN) * 8 / DOWNLINK_BPS) + LATENCY_BEST
    t_steady_total = N * t_steady_per_msg
    
    steady_goodput_mbps = (PAYLOAD * 8) / t_steady_per_msg / 1e6
    
    t_epoch = t_burst + t_steady_total
    amortized_goodput = (PAYLOAD * 8) / (t_epoch / N) / 1e6

    # Plot
    times = [0, t_burst, t_burst, t_epoch]
    goodputs = [0, 0, steady_goodput_mbps, steady_goodput_mbps]
    
    amortized_times = [0, t_epoch]
    amortized_goodputs = [amortized_goodput, amortized_goodput]

    fig, ax = plt.subplots(figsize=(6, 3.5))
    ax.plot(times, goodputs, "-", color="#d62728", label="Burst-then-Recover (Event-Driven)", linewidth=2)
    ax.plot(amortized_times, amortized_goodputs, "--", color="#1f77b4", label="Smooth Amortization (Modeled)", linewidth=2)
    
    ax.set_xlabel("Time since epoch start (seconds)")
    ax.set_ylabel("Instantaneous Goodput (Mbps)")
    ax.set_title(f"Instantaneous Goodput vs Time (R={R}, N={N})")
    ax.legend(loc="center right")
    ax.grid(True, linestyle=":", alpha=0.6)

    os.makedirs("figures", exist_ok=True)
    out_path = "figures/burst_goodput_time.pdf"
    png_path = "figures/burst_goodput_time.png"
    fig.savefig(out_path, format="pdf", bbox_inches="tight")
    fig.savefig(png_path, format="png", bbox_inches="tight", dpi=150)
    print(f"Wrote {out_path} and {png_path} (T_epoch={t_epoch:.3f}s, Amortized={amortized_goodput:.3f} Mbps)")

if __name__ == "__main__":
    main()
