# Known Limitations

This artifact is a research prototype. It does not provide production-grade
guarantees. Reviewers should understand these boundaries before interpreting
results.

## Verifier-Gated Entropy Numbers

**Status**: Open — the headline gating condition of the artifact.

**Description**: All security numbers derived from the pendulum (λ₁ ≈ 0.405,
KS ≈ 1.0–1.3 nats/s, 256-bit dt ≈ 136–176 s design-stage; spectrum-based KS
bound 179.5 s) are **design-stage measurements**, not final security claims.
They require independent verifier confirmation and carry two standing
caveats: the sampled λ_min is not a proven global minimum, and the
fixed-point discretization floor is unbounded above.

**Impact**:
- Manuscript numbers remain gated per the README caveat block (2026-09-06).
- The abstract's original "840.6 s epoch bound" is **retracted** (see
  `PROVENANCE.md` and the postmortem).

**Mitigation**: `scripts/validate_benettin.py` (float64 referee) and
`scripts/verify_metastability.py` encode the verify-before-trust discipline;
all committed rate data is tagged with its provenance and horizon.

## Fixed-Point Saturation Floor

**Status**: Inherent limitation, understood.

**Description**: Q32.32 states saturate at ±2^31. Long-horizon claims require
a bound on the discretization floor; a bounded-looking trajectory under
saturation is not evidence of a bounded system (this exact failure produced
the retracted "phantom attractor").

**Impact**: Any long-horizon fixed-point statistic is a fidelity ceiling, not
a confinement argument.

**Mitigation**: The bounded wrapped coupling is bounded *by construction*
(`atan2(sin Δθ, cos Δθ)`, ADR-004), independent of the fixed-point floor;
three dedicated wrapped-coupling tests (Jacobian slope-1 across branches,
on-cut behavior, spin-boundedness) run in `cargo test --release`.

## Simulator Visibility Model

**Status**: RESOLVED (2026-09-14) — geometry verified correct; metric semantics
clarified and constellation-level coverage now reported.

**Description**: The 24-satellite / 600 s window yields `visible_pct` ≈ 0.7–2%
in v2/v3 runs, which was flagged as suspiciously low for a 24-sat LEO
constellation. Investigation (`TestVisibilityPhysics`, added 2026-09-14)
verified the orbit/visibility geometry against the spherical-cap analytic
expectation: a 550 km satellite above 10° elevation covers (1−cos 15°)/2 ≈
1.696% of Earth's surface at any instant, and the measured per-satellite
visible fraction (1.8–2.5% over one orbital period from a mid-latitude
station) matches. The low `visible_pct` is therefore the correct
per-(satellite, time)-sample scale, not a geometry bug. The operationally
meaningful constellation-level numbers are now reported alongside it:
`any_visible_pct` ≈ 42–59% (≥1 satellite in view) and
`mean_sats_in_view` ≈ 0.42 over a 1200 s window. Baselines transmit at the
strongest-link moment rather than a fixed offset; visibility is
deterministic across seeds (fixed orbital phases), so window length, not
the seed, drives the fraction.

**Impact**: Unchanged — absolute goodput values depend on this visibility
model; comparisons *between* baselines under the same link model are
unaffected.

**Mitigation**: Claims are stated comparatively (chaosseal vs counter vs
BPSec under identical links), never on absolute Mbps against external systems.

## Wall-Clock Timing Noise

**Status**: Inherent limitation.

**Description**: The simulator records real wall-clock crypto timing.
Run-to-run jitter of ~0.1–1% in goodput is observed, with isolated outliers
(one 3% outlier of a 490 µs crypto measurement in Tier 2a).

**Impact**: R3-level quantities (see `REPRODUCIBILITY_LEVELS.md`) reproduce
within noise, not exactly.

**Mitigation**: Multi-seed reporting (5 seeds) and mean±std tables; all
claims are stated on differences exceeding the noise floor.

## BEE Covering-Set Simplification

**Status**: Known limitation.

**Description**: The BEE covering-set size uses a simplified heuristic
(`c(N,r) = ⌈log₂N / log₂ r⌉ · r` for r ≥ 2), not a full subset-difference
implementation (flagged in `docs/PROGRESS.md`).

**Impact**: Absolute broadcast sizes (2048 B at N=1024, r=8) may differ from
a complete Complete Subtree / Subset Difference scheme; the *comparative*
structure (join cost = revocation cost) is unaffected.

**Mitigation**: Stated explicitly in `docs/end_to_end_security_and_membership.md`
Part B; the join-vs-revoke equivalence follows from the construction, not the
constant factors.

## Hardware Benchmark

**Status**: CLOSED — measured on a physical STM32F4 Discovery
(STM32F407VG) on 2026-09-14; see
`firmware/stm32f4-bench/bench_results.json` (`bench_hw.json` for the raw
parsed capture).

**Description**: The `no_std` + `cortex-m-rt` port compiles the core_v2
kinematics and crypto sources **verbatim** (deltas limited to import
paths, libm for std-gated f64 math, and no OS-entropy nonce) with crypto
crates pinned to the `core_v2/Cargo.lock` versions. On the physical board
(PLL 168 MHz, Midas bring-up) all correctness gates pass: RFC 4231 HMAC
KAT byte-exact, AES-256-GCM roundtrip, HMAC commitment verify. Timing is
by SysTick at CLKSOURCE=CPU (ticks = true CPU cycles), validated by an
on-target dual-clock probe: SysTick 1,200,006 vs DWT CYCCNT 1,200,011
over the same 100k-iteration loop (0.0004% apart). Two capture runs are
byte-identical. Capture is serial-free: console lines are mirrored into
reserved SRAM2 and dumped via OpenOCD (`make firmware-capture`), since
the ST-LINK/V2 has no VCP.

**Measured numbers** (QEMU icount in parentheses):
- RK4 epoch step: 685,378 cycles (90,704 insns) — ~4.08 ms/step
- Lyapunov/Benettin step: 119,155 cycles (104,461 insns)
- Packet path (HKDF + AES-GCM 1024 B + HMAC): 421,176 cycles
  (53,396 insns) = **2.51 ms/packet**
- Epoch maintenance: 82.25 G cycles = **489.6 s = 40.8% of the 1200 s
  epoch** — the QEMU IPC=1 estimate (10.88 G ticks, 5.4%) was off by
  7.5x; the paper must quote the measured 40.8%.

**Impact**: Hardware cycles run 7.3–8.0x above QEMU instruction counts
(flash wait states, multi-cycle loads/multiplies); the libm-heavy
Lyapunov step is the outlier at 1.14x (FPU + hardware divide). The
feasibility conclusion survives: 2.51 ms/packet leaves a 195k
packet/epoch crypto budget, and maintenance is amortizable background
work — but the epoch share is **not** negligible and must be stated as
~41%, not ~5%.

**Residual boundaries**: Numbers are specific to the F407 @ 168 MHz with
the Midas PLL configuration (no hardware AES on this part); single-board
sample (deterministic workload makes this low-risk, but a second board
or an F7/H7 with hardware AES would shift absolute numbers); QEMU
instruction counts remain useful only as a deterministic reference and
relative-cost ranking, never as cycle estimates.

## Randomness and Reproducibility Boundary

**Status**: Design boundary.

**Description**: Netsim runs are exactly reproducible only given the same
seed, flags, and git commit (R2). Wall-clock-derived quantities never
reproduce bit-exactly (R3). There is no unseeded/stochastic component in the
canonical pipeline; all randomness derives from `--seed`.

**Impact**: Reviewers must not expect archived JSONs to regenerate
byte-identically from a replay unless the commit matches.

**Mitigation**: `REPRODUCIBILITY_LEVELS.md` classifies every result class;
each archived JSON embeds `git_commit`, `rng_seed`, and `parameters`.
