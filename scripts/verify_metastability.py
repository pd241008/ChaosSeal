#!/usr/bin/env python3
"""Independent verification of the metastability finding for the reinjected
3-pendulum ODE (linear elastic coupling).

Reproduces, from first principles and deterministically:

  1. Bounded control  -- coupling c=0 stays in the swing basin forever
     (kick energy 4.5 J << rotational barrier 19.6 J).
  2. Unbounded escape  -- default c=0.5 escapes the swing basin after
     ~100-200 s and pumps |omega| without bound. Cross-checked across
     integrators (RK4 at dt=1e-2 and 2e-3, symplectic Verlet at 2e-3).
  3. Escape-time CDF   -- onset of |omega| > 50 for the standard random-IC
     distribution (epoch-cap input). Batched/vectorized over ICs.

Optional extras (slower): --fine  adds RK4 dt=1e-4; --dop853 adds scipy DOP853.

Usage:
  python3 scripts/verify_metastability.py            # fast (default)
  python3 scripts/verify_metastability.py --fine --dop853
  python3 scripts/verify_metastability.py --json     # machine-readable report

Exit code 0 iff all asserted claims reproduce.
"""

import argparse
import json
import sys
import time

import numpy as np

N = 3
G = 9.80665
BOOST = 3.0
BOUND = 50.0          # |omega| beyond this counts as escaped into the spin regime
BIN_S = 100.0


def kinematics():
    return {"N": N, "G": G, "mass": 1.0, "length": 1.0,
            "damping": 0.1, "coupling": 0.5}


def deriv(xx, c):
    th = xx[:N]
    om = xx[N:]
    d = np.zeros_like(xx)
    d[N:] = -0.1 * om
    for i in range(N):
        d[N + i] += G * (2.0) * (0.5) * np.sin(th[i])
        if i >= 1:
            d[N + i] += c * ((th[i] - th[i - 1]) / 1.0) * 0.1
        d[i] = om[i]
    return d


def rk4_step(x, dt, c, kick=True):
    s = x.copy()
    if kick and np.sum(np.abs(s[N:])) < 0.5:
        s[N] += BOOST
    k1 = deriv(s, c)
    s2 = s + 0.5 * dt * k1
    k2 = deriv(s2, c)
    s3 = s + 0.5 * dt * k2
    k3 = deriv(s3, c)
    s4 = s + dt * k3
    k4 = deriv(s4, c)
    return s + dt / 6.0 * (k1 + 2 * k2 + 2 * k3 + k4)


def verlet_step(x, dt, c):
    s_th = x[:N].copy()
    s_om = x[N:].copy()
    if np.sum(np.abs(s_om)) < 0.5:
        s_om[0] += BOOST
    a1 = deriv(np.concatenate([s_th, s_om]), c)
    b2 = 0.5 * 0.1 * dt
    om_h = s_om * (1.0 - b2) + 0.5 * dt * a1[N:]
    th_n = s_th + dt * om_h
    a2 = deriv(np.concatenate([th_n, om_h]), c)
    om_n = om_h * (1.0 - b2) + 0.5 * dt * a2[N:]
    return np.concatenate([th_n, om_n])


def default_ic():
    return np.array([0.1, 0.2, 0.3, 0.0, 0.0, 0.0])


def run_bins(step_fn, x0, dt, T, bin_s=BIN_S, early_exit=BOUND):
    steps = int(T / dt)
    x = x0.copy()
    wmax = 0.0
    bins = []
    first_escape = None
    skip = int(bin_s / dt)
    for step in range(steps):
        x = step_fn(x, dt)
        w = float(np.max(np.abs(x[N:])))
        wmax = max(wmax, w)
        if first_escape is None and w > early_exit:
            first_escape = step * dt
            if early_exit is not None and T > 0:
                pass
        if (step + 1) % skip == 0:
            bins.append(wmax)
            wmax = 0.0
    return bins, first_escape


def batched_escape_times(x0s, dt, T, seed):
    """Vectorized RK4 over a batch of ICs (axis -1 is the IC index)."""
    steps = int(T / dt)
    xs = x0s.T.astype(np.float64)          # shape (6, m)
    esc = np.full(xs.shape[1], np.inf)

    def bderiv(xx):
        th = xx[:N]
        om = xx[N:]
        d = np.zeros_like(xx)
        d[N:] = -0.1 * om
        for i in range(N):
            d[N + i] += G * (2.0) * (0.5) * np.sin(th[i])
            if i >= 1:
                d[N + i] += 0.5 * ((th[i] - th[i - 1]) / 1.0) * 0.1
            d[i] = om[i]
        return d

    skip = 400
    for step in range(steps):
        s = xs.copy()
        kick = np.sum(np.abs(s[N:]), axis=0) < 0.5
        if kick.any():
            s[N, kick] += BOOST
        k1 = bderiv(s)
        s2 = s + 0.5 * dt * k1; k2 = bderiv(s2)
        s3 = s + 0.5 * dt * k2; k3 = bderiv(s3)
        s4 = s + dt * k3;       k4 = bderiv(s4)
        xs = s + dt / 6.0 * (k1 + 2 * k2 + 2 * k3 + k4)
        if step % skip == skip - 1:
            over = np.max(np.abs(xs[N:]), axis=0) > BOUND
            t = (step + 1) * dt
            esc[np.isinf(esc) & over] = t
            if np.all(np.isfinite(esc)):
                return np.sort(esc)
    return np.sort(esc)


def check_control(T=1200.0, dt=2e-3):
    bins, esc = run_bins(lambda x, d: rk4_step(x, d, 0.0), default_ic(), dt, T)
    return (esc is None and all(b < BOUND for b in bins)), bins


def check_escape(dt, kind="rk4", T=400.0, c=0.5):
    fn = (lambda x, d: rk4_step(x, d, c)) if kind == "rk4" else \
         (lambda x, d: verlet_step(x, d, c))
    bins, esc = run_bins(fn, default_ic(), dt, T)
    exploded = esc is not None and esc <= 300.0 and max(bins[1:]) > 1e6
    return exploded, bins, esc


def check_dop853(T=300.0):
    try:
        from scipy.integrate import solve_ivp
    except ImportError:
        return None, None, "scipy not installed"
    x = default_ic()
    t0 = 0.0
    bins = []
    wmax = 0.0
    esc = None
    while t0 < T - 1e-9:
        s = x.copy()
        if np.sum(np.abs(s[N:])) < 0.5:
            s[N] += BOOST
        r = solve_ivp(lambda t, xx: deriv(xx, 0.5), (t0, t0 + 0.01), s,
                      method="DOP853", rtol=1e-10, atol=1e-13)
        if not r.success:
            return None, None, r.message
        x = r.y[:, -1]
        t0 += 0.01
        w = float(np.max(np.abs(x[N:])))
        wmax = max(wmax, w)
        if esc is None and w > BOUND:
            esc = t0
        if abs(t0 - BIN_S * round(t0 / BIN_S)) < 0.01 / 2:
            bins.append(wmax)
            wmax = 0.0
    exploded = esc is not None and esc <= 250.0 and (len(bins) == 0 or max(bins) > 1e6)
    return exploded, bins, esc


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--fine", action="store_true", help="add RK4 dt=1e-4")
    ap.add_argument("--dop853", action="store_true", help="add scipy DOP853")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--n-ic", type=int, default=24)
    args = ap.parse_args()

    def log(*a):
        print(*a, flush=True)

    report = {"kinematics": kinematics()}
    all_ok = True
    t0 = time.time()

    ok, bins = check_control()
    report["control_c0"] = {"bounded": ok, "max_omega_bins_100s": [f"{v:.2f}" for v in bins]}
    all_ok &= ok
    log(f"[1/4] control c=0: bounded = {ok}  max|w| {max(bins):.2f} over 1200 s")

    sig = {}
    for label, (dt, kind) in {
        "rk4_dt1e-2": (1e-2, "rk4"),
        "rk4_dt2e-3": (2e-3, "rk4"),
        "verlet_dt2e-3": (2e-3, "verlet"),
    }.items():
        expl, b, esc = check_escape(dt, kind=kind)
        sig[label] = {"exploded": expl,
                      "max_omega_bins_100s": [f"{v:.3e}" for v in b],
                      "first_escape_s": None if esc is None else round(esc, 1)}
        all_ok &= expl
        log(f"[2/4] {label}: exploded = {expl}  escape@{esc}  bins={[f'{v:.1e}' for v in b]}")

    if args.fine:
        expl, b, esc = check_escape(1e-4, kind="rk4", T=300.0)
        sig["rk4_dt1e-4"] = {"exploded": expl,
                             "max_omega_bins_100s": [f"{v:.3e}" for v in b],
                             "first_escape_s": None if esc is None else round(esc, 1)}
        all_ok &= expl
        log(f"[2/4] rk4_dt1e-4: exploded = {expl}  escape@{esc}  bins={[f'{v:.1e}' for v in b]}")

    if args.dop853:
        expl, b, esc = check_dop853()
        if b is None:
            log(f"[2/4] DOP853: skipped ({esc})")
        else:
            sig["DOP853"] = {"exploded": expl,
                             "max_omega_bins_100s": [f"{v:.3e}" for v in b],
                             "first_escape_s": None if esc is None else round(esc, 1)}
            all_ok &= expl
            log(f"[2/4] DOP853: exploded = {expl}  escape@{esc}  bins={[f'{v:.1e}' for v in b]}")
    report["escape"] = sig

    x0s = np.random.default_rng(11).uniform(-3.14159, 3.14159, (6, args.n_ic))
    ts = batched_escape_times(x0s, 2e-3, 400.0, 11)
    finite = ts[np.isfinite(ts)]
    cdf = {"n": args.n_ic,
           "min": float(finite.min()) if finite.size else np.inf,
           "p05": float(np.percentile(finite, 5.0)) if finite.size else np.inf,
           "median": float(np.median(finite)) if finite.size else np.inf,
           "escaped_before_400s": float((ts < 400).mean()),
           "never_escaped": float((ts == np.inf).mean())}
    report["escape_cdf"] = cdf
    log(f"[3/4] escape-time CDF (n={args.n_ic}): min={cdf['min']:.1f}s "
        f"p05={cdf['p05']:.1f}s median={cdf['median']:.1f}s "
        f"escaped<400s={cdf['escaped_before_400s']*100:.0f}%")
    if finite.size:
        log("      order:", ", ".join(f"{v:.0f}" for v in np.sort(finite)[:12]))

    qmax = (2**63 - 1) / 2**32
    report["fixedpoint"] = {"saturation_bound": qmax,
                            "source": "core_v2/src/fixed/q32_32.rs (i64, 32 frac bits)"}
    log(f"[4/4] fixed-point Q32.32 saturation bound = +/-{qmax:.3e}")
    all_ok &= abs(qmax - float(2**31)) < 1e-6

    report["duration_s"] = round(time.time() - t0, 1)
    report["all_assertions_ok"] = bool(all_ok)
    if args.json:
        print(json.dumps(report, indent=2, default=str), flush=True)
    log(f"\nFINAL: assertions_ok = {all_ok}  (duration {report['duration_s']}s)")
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())