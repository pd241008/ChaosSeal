#!/usr/bin/env python3
"""Fast numpy float64 reference for the pendulum Benettin estimator, to
adjudicate the long-horizon divergence between the Rust Q32.32 estimator
(which converges to lambda1 ~ 1.46 nats/s) and the slow pure-python float64
reference (lambda1 ~ 0.22 at 2000 s, apparently still drifting).

Same ODE + reinjection + GS tangent machinery, vectorized. Usage:
    python3 scripts/reference_spectrum_long_horizon.py [horizon_s [IC seed]]
"""
import sys, math
import numpy as np

N = 3
G = 9.80665
DT = 0.01
RINT = 10

def run(T, x0, with_inject=True):
    steps = int(T / DT)
    # state layout: [theta_0..2, omega_0..2]
    def deriv(x):
        n = N
        theta = x[:n]
        omega = x[n:]
        d = np.zeros_like(x)
        d[n:] = -0.1 * omega
        for i in range(n):
            tg = G * (1.0 * 2.0) * (1.0 / 2.0) * np.sin(theta[i])
            tc = 0.0
            if i >= 1:
                tc = 0.5 * ((theta[i] - theta[i-1]) / 1.0) * 0.1
            d[n + i] += (tg + tc) / (1.0 * 1.0 * 1.0)
            d[i] = omega[i]
        return d

    def jacobian(x):
        n = N
        J = np.zeros((2 * n, 2 * n))
        for i in range(n):
            J[i, n + i] = 1.0
            dom = G * (1.0 * 2.0) * (1.0 / 2.0) * np.cos(x[i])
            if i >= 1:
                dom += 0.5 * 0.1 / 1.0
                J[n + i, i - 1] = -0.5 * 0.1 / 1.0
            J[n + i, i] = dom
            J[n + i, n + i] = -0.1
        return J

    x = np.array(x0, dtype=np.float64)
    Tn = np.eye(2 * N)[:3, :].copy()  # 3 tangent vectors (full 6-D)
    logsum = np.zeros(3)

    kick_count = 0
    bins = []
    wmax = 0.0
    for step in range(steps):
        if with_inject:
            s = x.copy()
            if np.sum(np.abs(s[N:])) < 0.5:
                s[N] += 3.0
                kick_count += 1
        else:
            s = x.copy()
        k1 = deriv(s)
        s2 = s + 0.5 * DT * k1; k2 = deriv(s2)
        s3 = s + 0.5 * DT * k2; k3 = deriv(s3)
        s4 = s + DT * k3;       k4 = deriv(s4)
        x = s + (DT / 6.0) * (k1 + 2 * k2 + 2 * k3 + k4)
        wmax = max(wmax, float(np.max(np.abs(x[N:]))))
        if (step + 1) % (10 * 100) == 0:  # every 100 s
            bins.append(wmax)
            wmax = 0.0
        J = jacobian(x)
        Tn = Tn + (Tn @ J.T) * DT

        if step > 0 and step % RINT == 0:
            norms = np.sqrt(np.sum(Tn * Tn, axis=1))
            for k in range(3):
                if norms[k] > 0:
                    Tn[k] /= norms[k]
                    logsum[k] += math.log(norms[k])
            for i in range(1, 3):
                for j in range(i):
                    dot = float(Tn[i] @ Tn[j])
                    Tn[i] -= dot * Tn[j]
                    ni = math.sqrt(float(Tn[i] @ Tn[i]))
                    if ni > 0:
                        Tn[i] /= ni
    lam = logsum / T
    return lam, kick_count, math.sqrt(float(x[3] ** 2 + x[4] ** 2 + x[5] ** 2))

def main():
    T = float(sys.argv[1]) if len(sys.argv) > 1 else 2000.0
    seed = int(sys.argv[2]) if len(sys.argv) > 2 else 7
    rng = np.random.default_rng(seed)
    x0 = np.array([0.1, 0.2, 0.3, 0.0, 0.0, 0.0])
    lam, kicks, wmag = run(T, x0)
    ks = float(np.sum(lam[lam > 0]))
    print(f"det IC:  T={T:.0f}s lam1={lam[0]:.5f} lam2={lam[1]:.5f} lam3={lam[2]:.5f} "
          f"ks_pos={ks:.5f} kicks={kicks} |w|_end={wmag:.2f}")
    if len(sys.argv) > 2:
        # random IC from the attractor-style [-pi, pi]^6 box
        x0r = rng.uniform(-3.14159, 3.14159, 6)
        lam, kicks, wmag = run(T, x0r)
        ks = float(np.sum(lam[lam > 0]))
        print(f"rnd IC:  T={T:.0f}s lam1={lam[0]:.5f} lam2={lam[1]:.5f} lam3={lam[2]:.5f} "
              f"ks_pos={ks:.5f} kicks={kicks} |w|_end={wmag:.2f}")

if __name__ == "__main__":
    main()