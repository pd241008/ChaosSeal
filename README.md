# Dynamic Node Revocation and Chaos-Derived Key Rotation in LEO Satellite Swarms: A Dual-Layer Cryptographic Protocol

[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.22005854.svg)](https://doi.org/10.5281/zenodo.22005854)
![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![Go](https://img.shields.io/badge/Go-00ADD8?style=flat&logo=go&logoColor=white)
![Python](https://img.shields.io/badge/Python-3776AB?style=flat&logo=python&logoColor=white)

> **Reference Implementation for the IEEE Ad Hoc Networks Submission**  
> **Authors:** Prathmesh Desai, Avinash Sastry  

## Abstract

Low Earth Orbit (LEO) satellite swarms require secure, high-bandwidth, and delay-tolerant communications, yet traditional Public Key Infrastructure (PKI) is poorly suited to this environment because of the high latency of long-fat-network (LFN) links and the prohibitive overhead of re-keying an entire swarm after a single node compromise. This paper proposes the Chaotic Exclusion Protocol (CEP), a dual-layer architecture that combines Broadcast Exclusion Encryption (BEE) for instant, sublinear-cost node revocation with a chaos-derived, hardware-portable key-rotation mechanism for the data-transmission layer. Rather than using a chaotic trajectory directly as an XOR keystream—a design repeatedly broken in prior chaos-cryptography literature via phase-space reconstruction attacks—CEP uses the chaotic pendulum purely as a deterministic entropy source for an HKDF-based key schedule feeding standard AES-256-CTR encryption. Cross-platform floating-point divergence is eliminated using fixed-point (Q32.32) arithmetic, and synchronization is verified with an exact HMAC commitment rather than a similarity threshold. We further replace the original SMTP-based store-and-forward design with the CCSDS Bundle Protocol (BPv7) and BPSec, aligning the delay-tolerant transport layer with current space-networking standards. We present the protocol architecture, an explicit threat model, formal confidentiality and entropy arguments—including a concrete, measured epoch-duration bound of 840.6 s derived from the pendulum's estimated dominant Lyapunov exponent—and a multi-seed simulation evaluation showing that CEP delivers higher goodput than a BPSec baseline when fewer than approximately 6% of the swarm is simultaneously revoked, with a predictable, low-variance crossover past which BPSec's fixed per-bundle overhead becomes more efficient than CEP's revocation-amortized cost; an event-driven burst-then-recover model confirms the amortized goodput prediction holds on time-average.

### Highlights
- Proposes a dual-layer protocol combining Broadcast Exclusion Encryption with a chaos-derived key schedule for LEO satellite swarm security.
- Replaces direct chaotic-XOR keystream generation with an HKDF-to-AES-256-CTR construction, avoiding known phase-space reconstruction attacks on chaos ciphers.
- Resolves cross-processor floating-point divergence using Q32.32 fixed-point arithmetic and an exact HMAC-based synchronization check.
- Aligns the delay-tolerant transport layer with the CCSDS Bundle Protocol (BPv7) and BPSec rather than a legacy email-based transport.
- Identifies an empirical crossover: CEP outperforms a BPSec baseline in goodput below approximately 6% simultaneous revocation, with a measurable, predictable degradation point beyond it.

---

## Citation

If you build on this work or use the reference implementation, please cite the paper:

```bibtex
@article{desai2026dynamic,
  title={Dynamic Node Revocation and Chaos-Derived Key Rotation in LEO Satellite Swarms: A Dual-Layer Cryptographic Protocol},
  author={Desai, Prathmesh and Sastry, Avinash},
  journal={IEEE Ad Hoc Networks (Submitted)},
  year={2026}
}
```

---

## Architecture & Tech Stack

This repository implements the full experimental pipeline across a polyglot architecture:

1. **Protocol Core (Rust)**: Located in `core/`. This handles all protocol logic. We use deterministic Q32.32 fixed-point Runge-Kutta solvers alongside vetted `hkdf` and `aes` crates. This ensures absolute memory safety, performance, and cross-platform bit-exact execution (critical for eliminating floating-point divergence in Lyapunov estimation).
2. **Network Simulator (Go)**: Located in `netsim/`. This Go harness simulates the LEO swarm, modeling intermittent visibility windows, elevation-dependent link latency, and Gilbert-Elliott loss bursts. It also includes the standard BPv7/BPSec and TLS 1.3 baselines used in the paper for comparison.

---

## Build and Installation

For researchers replicating the results, the friction to build the environment is kept intentionally low for standard Linux and WSL2 environments.

**Prerequisites:**
- Rust $\geq$ 1.75
- Go $\geq$ 1.22
- Python $\geq$ 3.10 (for plotting graphs)

**Compilation:**
To build the release binaries, execute the following from the root directory:

```bash
# Build the Rust cryptographic core
cargo build --release --manifest-path core/Cargo.toml

# Build the Go network simulator
go build -o chaoseal-sim ./netsim
```

*(Note: Ensure the Rust shared library bindings are available to the Go environment if using CGO)*.

---

## Reproducing the Paper's Evaluation

The evaluation data and figures in the draft can be exactly reproduced using the CLI. The parameters used in our paper map directly to these commands. 

### Figure 5 & Table 2: Goodput Crossover
Run a full sweep of $R \in \{1, 2, 4, 8, 16, 32, 64, 128, 256, 512\}$ with $N=1024$ nodes over a 1200s epoch across 5 RNG seeds:

```bash
./chaoseal-sim --mode sweep --N 1024 --epoch 1200 --seeds 5
```

### Figure 4: Latency Distribution
Trigger the HMAC mismatch and output the latency logs for the BEE Correction Vector propagation:

```bash
./chaoseal-sim --mode latency-dist --trigger-hmac-mismatch true
```

### Figure 7: Burst-then-Recover
Run the event-driven instantaneous goodput model for $\vert{}R\vert{}=512$:

```bash
./chaoseal-sim --mode burst-recover --R 512
```

---

## Repository Navigation

For reviewers and researchers, the repository is structured to transparently map to claims made in the paper:

```
.
├── core/                        # Protocol Core (Rust)
│   ├── src/crypto/
│   │   ├── hmac_sha256.rs       # 📍 Exact-match HMAC commitment logic
│   │   └── aes_ctr.rs           # 📍 HKDF key derivation & AES-CTR
│   ├── src/bee/                 # Subset-difference key tree & covering set
│   └── src/kinematics/          # Q32.32 fixed-point ODE solver
├── netsim/                      # Network Simulator (Go)
│   └── main.go                  # CLI entry point for the simulation harness
├── analysis/                    # Python scripts for figures and stats
└── results/                     # Raw JSON output from simulations
```

### Key Cryptographic Mechanisms

- **Exact-Match HMAC Commitment:** The logic for verifying the chaotic state using an exact HMAC check (resolving the similarity threshold issue of prior chaotic protocols) is located in [`core/src/crypto/hmac_sha256.rs`](file:///root/workspace/workspace/03-Code/Projects/Legacy/ChaosSeal/core/src/crypto/hmac_sha256.rs).
- **HKDF Key Derivation:** The secure generation of the AES-CTR keystream via the HKDF construction (avoiding phase-space reconstruction) is implemented in [`core/src/crypto/aes_ctr.rs`](file:///root/workspace/workspace/03-Code/Projects/Legacy/ChaosSeal/core/src/crypto/aes_ctr.rs).

---

<!-- Hidden Paper Figures for Internal Reference -->
<!-- 
<img src="./analysis/figures/throughput.pdf" alt="Raw Throughput" width="400"/>
<img src="./analysis/figures/goodput.pdf" alt="Effective Goodput" width="400"/>
<img src="./analysis/figures/goodput_vs_r.pdf" alt="Goodput Degradation vs R" width="400"/>
<img src="./analysis/figures/bee_size_vs_r.pdf" alt="BEE Size vs R" width="400"/>
<img src="./analysis/figures/resync_latency.pdf" alt="Resync Latency" width="400"/>
-->

## License

This reference implementation is licensed under the MIT License.
