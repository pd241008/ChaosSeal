#!/usr/bin/env python3
"""
Explore bounded-coupling alternatives to the linear elastic coupling that
was shown (verify_metastability.py) to make the reinjected 3-pendulum system
escape into unbounded energy growth within ~17-130s.

Tests three candidate couplings, each swept over a few strengths:
  A) angle-wrapped linear:   tau_c = C * wrap(theta_i - theta_j) * 0.1
  B) tanh-saturating:        tau_c = C * tanh((theta_i - theta_j)/scale) * 0.1
  C) sin-coupling, finer sweep than the original two-point check:
                             tau_c = C * sin(theta_i - theta_j) * 0.1

For every (coupling_type, strength) pair, runs the same three checks used to
establish the original metastability finding:
  1. Long-horizon boundedness  -- T=20000s (~150x the original ~130s escape
     horizon), RK4, checking |omega| never exceeds BOUND.
  2. Two-trajectory Lyapunov exponent (standard renormalization method,
     no Jacobian needed) from the deterministic default IC.
  3. Basin-independence sweep -- lambda1 across N_IC random initial
     conditions; reports fraction positive (chaotic) and whether the
     distribution is bimodal (checked via a simple two-cluster gap test,
     not just eyeballing it).

Usage:
  python3 explore_bounded_coupling.py                # fast (default N_IC=40)
  python3 explore_bounded_coupling.py --n-ic 200      # more thorough basin check
  python3 explore_bounded_coupling.py --json > results.json

Exit code 0 always (this is exploratory, not a pass/fail gate); read the
printed table / JSON to see which (if any) candidate is both bounded and
robustly chaotic.
"""
import argparse
import json
import sys

import numpy as np

N = 3
G = 9.80665
BOOST = 3.0
BOUND = 50.0            # |omega| beyond this = escaped, same threshold as before
LONG_T = 20000.0        # ~150x the original ~130s escape horizon
LONG_DT = 5e-3
LYAP_T = 300.0
LYAP_DT = 2e-3
D0 = 1e-8
RENORM_EVERY = 1.0      # seconds between Lyapunov renormalizations


def wrap(dtheta):
    return np.arctan2(np.sin(dtheta), np.cos(dtheta))


def make_deriv(kind, C):
    """Returns a deriv(x) -> xdot function for the given coupling kind/strength.

    x may be a flat 6-vector or an (K, 6) batch; the ODE is evaluated on the
    last axis so both shapes share exactly the same math.
    """
    strength = C[0] if isinstance(C, tuple) else C
    if kind == "wrapped_linear":
        def g(dth):
            return wrap(dth)
    elif kind == "tanh":
        scale = C[1]
        def g(dth):
            return np.tanh(dth / scale)
    elif kind == "sin":
        def g(dth):
            return np.sin(dth)
    else:
        raise ValueError(kind)

    def deriv(x):
        th = x[..., :N]
        om = x[..., N:]
        omdot = -0.1 * om
        omdot[..., 0] += G * np.sin(th[..., 0])
        for i in range(1, N):
            omdot[..., i] += G * np.sin(th[..., i]) + strength * g(th[..., i] - th[..., i - 1]) * 0.1
        return np.concatenate([om, omdot], axis=-1)
    return deriv


def rk4_step(deriv, x, dt, kick=True):
    s = x.copy()
    if kick and np.sum(np.abs(s[N:])) < 0.5:
        s[N] += BOOST
    k1 = deriv(s)
    k2 = deriv(s + 0.5 * dt * k1)
    k3 = deriv(s + 0.5 * dt * k2)
    k4 = deriv(s + dt * k3)
    return s + dt / 6.0 * (k1 + 2 * k2 + 2 * k3 + k4)


def default_ic():
    return np.array([0.1, 0.2, 0.3, 0.0, 0.0, 0.0])


def check_bounded(deriv, T=LONG_T, dt=LONG_DT):
    x = default_ic()
    steps = int(T / dt)
    wmax = 0.0
    escaped_at = None
    check_every = max(1, int(1.0 / dt))
    for step in range(steps):
        x = rk4_step(deriv, x, dt)
        if step % check_every == 0:
            w = float(np.max(np.abs(x[N:])))
            wmax = max(wmax, w)
            if w > BOUND and escaped_at is None:
                escaped_at = step * dt
                break
    return {"bounded": escaped_at is None, "max_omega": wmax, "escaped_at_s": escaped_at}


def lyapunov_two_traj(deriv, x0, T=LYAP_T, dt=LYAP_DT, d0=D0, renorm_every=RENORM_EVERY):
    x1 = x0.copy()
    x2 = x0.copy()
    x2[0] += d0
    steps = int(T / dt)
    renorm_steps = max(1, int(renorm_every / dt))
    log_sum = 0.0
    n_renorm = 0
    for step in range(steps):
        x1 = rk4_step(deriv, x1, dt)
        x2 = rk4_step(deriv, x2, dt)
        if np.max(np.abs(x1[N:])) > BOUND or np.max(np.abs(x2[N:])) > BOUND:
            return None  # escaped mid-measurement; undefined for this design
        if (step + 1) % renorm_steps == 0:
            diff = x2 - x1
            dist = np.linalg.norm(diff)
            if dist == 0:
                continue
            log_sum += np.log(dist / d0)
            n_renorm += 1
            x2 = x1 + diff * (d0 / dist)
    if n_renorm == 0:
        return None
    return log_sum / (n_renorm * renorm_every)


def rk4_batch(deriv, X, dt, kick=True):
    """One RK4 step for a (K,6) batch of trajectories, reinjection per row."""
    K = X.shape[0]
    s = X.copy()
    if kick:
        wsum = np.sum(np.abs(s[:, N:]), axis=1)
        s[wsum < 0.5, N] += BOOST
    k1 = deriv(s)
    k2 = deriv(s + 0.5 * dt * k1)
    k3 = deriv(s + 0.5 * dt * k2)
    k4 = deriv(s + dt * k3)
    return s + dt / 6.0 * (k1 + 2 * k2 + 2 * k3 + k4)


def basin_sweep(deriv, n_ic, seed=7, window_s=150.0, dump_path=None):
    rng = np.random.default_rng(seed)
    thetas = rng.uniform(-3.14159, 3.14159, (n_ic, N))
    omegas = rng.uniform(-0.5, 0.5, (n_ic, N))
    X1 = np.concatenate([thetas, omegas], axis=1)
    X2 = X1.copy()
    X2[:, 0] += D0
    lam = np.full(n_ic, np.nan)
    log_accum = np.zeros(n_ic)
    n_renorm = np.zeros(n_ic)
    active = np.ones(n_ic, dtype=bool)
    dt = LYAP_DT
    renorm_steps = max(1, int(RENORM_EVERY / dt))
    steps = int(window_s / dt)
    for step in range(steps):
        if not active.any():
            break
        X1[active] = rk4_batch(deriv, X1[active], dt)
        X2[active] = rk4_batch(deriv, X2[active], dt)
        wmax = np.maximum(np.max(np.abs(X1[:, N:]), axis=1),
                          np.max(np.abs(X2[:, N:]), axis=1))
        escaped = active & (wmax > BOUND)
        if escaped.any():
            active &= ~escaped
            lam[escaped] = np.nan
        if (step + 1) % renorm_steps == 0:
            diff = X2[active] - X1[active]
            dist = np.linalg.norm(diff, axis=1)
            nz = dist > 0
            log_accum[active] += np.where(nz, np.log(np.where(nz, dist, 1.0) / D0), 0.0)
            n_renorm[active] += 1.0
            rescale = np.ones_like(dist)
            rescale[nz] = D0 / dist[nz]
            X2[active] = X1[active] + diff * rescale[:, None]
    ok = active & (n_renorm > 0)
    lam[ok] = log_accum[ok] / (n_renorm[ok] * RENORM_EVERY)

    if dump_path:
        with open(dump_path, "w") as f:
            f.write("idx,theta1,theta2,theta3,omega1,omega2,omega3,lambda1\n")
            for i in range(n_ic):
                lam_s = "escaped" if np.isnan(lam[i]) else f"{lam[i]:.8f}"
                f.write(f"{i},{thetas[i,0]:.9f},{thetas[i,1]:.9f},{thetas[i,2]:.9f},"
                        f"{omegas[i,0]:.9f},{omegas[i,1]:.9f},{omegas[i,2]:.9f},{lam_s}\n")

    lambdas = lam[~np.isnan(lam)]
    if len(lambdas) == 0:
        return {"n_valid": 0, "frac_chaotic": None, "bimodal": None, "mean": None}
    frac_chaotic = float((lambdas > 0.01).mean())
    # simple bimodality check: sort, find the largest gap, see if it splits
    # the data into two clusters each >15% of the total
    sl = np.sort(lambdas)
    if len(sl) >= 6:
        gaps = np.diff(sl)
        gi = int(np.argmax(gaps))
        left, right = gi + 1, len(sl) - (gi + 1)
        bimodal = (left / len(sl) > 0.15) and (right / len(sl) > 0.15) and \
                  (gaps[gi] > 3 * np.median(gaps) if np.median(gaps) > 0 else gaps[gi] > 0.05)
    else:
        bimodal = False
    return {
        "n_valid": int(len(lambdas)),
        "n_escaped_during_measurement": int(n_ic - len(lambdas)),
        "frac_chaotic": frac_chaotic,
        "bimodal": bool(bimodal),
        "mean": float(lambdas.mean()),
        "min": float(lambdas.min()),
        "max": float(lambdas.max()),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n-ic", type=int, default=40)
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--skip-long", action="store_true",
                     help="skip the 20000s boundedness check (slow); use for a quick look")
    ap.add_argument("--kinds", default=None,
                     help="comma-separated subset of {wrapped_linear,tanh,sin} to run "
                          "(default: all)")
    ap.add_argument("--window", type=float, default=150.0,
                     help="basin-sweep measurement window in seconds (default 150)")
    ap.add_argument("--dump-csv", default=None,
                     help="path to write per-IC (theta,omega,lambda1) rows for the "
                          "(single) candidate being run")
    args = ap.parse_args()

    candidates = [
        ("wrapped_linear", 0.5, "wrapped_linear c=0.5 (direct fix: wrap the original coupling)"),
        ("wrapped_linear", 1.0, "wrapped_linear c=1.0"),
        ("tanh", (0.5, 1.0), "tanh c=0.5 scale=1.0"),
        ("tanh", (1.0, 0.5), "tanh c=1.0 scale=0.5 (tighter saturation)"),
        ("tanh", (2.0, 1.0), "tanh c=2.0 scale=1.0"),
        ("sin", 0.8, "sin c=0.8 (between the original 0.5 and 2.0 data points)"),
        ("sin", 1.0, "sin c=1.0"),
        ("sin", 1.2, "sin c=1.2"),
        ("sin", 1.5, "sin c=1.5"),
    ]

    results = []
    for kind, C, label in candidates:
        if args.kinds is not None and kind not in args.kinds.split(","):
            continue
        deriv = make_deriv(kind, C)
        print(f"\n=== {label} ===", flush=True)

        bounded = {"bounded": None, "max_omega": None, "escaped_at_s": None}
        if not args.skip_long:
            bounded = check_bounded(deriv)
            print(f"  long-horizon (T={LONG_T:.0f}s): bounded={bounded['bounded']} "
                  f"max|w|={bounded['max_omega']:.2f} escaped_at={bounded['escaped_at_s']}")

        lam_default = lyapunov_two_traj(deriv, default_ic())
        print(f"  default-IC lambda1 (T={LYAP_T:.0f}s): "
              f"{'ESCAPED' if lam_default is None else f'{lam_default:.4f}'}")

        basin = basin_sweep(deriv, args.n_ic, window_s=args.window, dump_path=args.dump_csv)
        print(f"  basin sweep (n={args.n_ic}): valid={basin['n_valid']} "
              f"escaped_during_measurement={basin.get('n_escaped_during_measurement')} "
              f"frac_chaotic={basin['frac_chaotic']} bimodal={basin['bimodal']} "
              f"mean_lambda1={basin['mean']}")

        results.append({
            "label": label, "kind": kind, "C": C,
            "long_horizon": bounded, "default_ic_lambda1": lam_default,
            "basin_sweep": basin,
        })

    print("\n\n=== SUMMARY (candidate is worth pursuing iff bounded=True, "
          "lambda1 clearly positive, frac_chaotic high, bimodal=False) ===")
    for r in results:
        b = r["long_horizon"]["bounded"]
        lam = r["default_ic_lambda1"]
        fc = r["basin_sweep"]["frac_chaotic"]
        bm = r["basin_sweep"]["bimodal"]
        verdict = "PROMISING" if (b in (True, None) and lam is not None and lam > 0.01
                                   and fc is not None and fc > 0.9 and not bm) else "reject"
        print(f"  [{verdict:9s}] {r['label']:55s} bounded={b} lambda1={lam} "
              f"frac_chaotic={fc} bimodal={bm}")

    if args.json:
        print(json.dumps(results, indent=2, default=str))

    return 0


if __name__ == "__main__":
    sys.exit(main())