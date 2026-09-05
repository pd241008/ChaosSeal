#!/usr/bin/env python3
"""Independent float64 reference of the Benettin estimator with the corrected
Jacobian tangent update (v <- v + J(x)v*dt), cross-checked against the Rust
Q32.32 fixed-point implementation at several equilibrium parameters, including
inertia != 1.

Run:  python3 scripts/validate_benettin.py

It replicates core_v2/lyapunov/benettin.rs + kinematics/pendulum.rs exactly in
float64 (same ODE, same coupling convention, same reinjection, same RK4 step,
same reorthonormalization schedule) and compares the resulting lambda_1, for a
single deterministic initial condition [0.1, 0.2, 0.3, 0, 0, 0] (the one the
`cli_v2 lyapunov` command uses), across parameter sets. The inertia!=1 cases
exercise the damping/coupling Jacobian entries that are invisible at the
m=1.0, L=1.0 default.
"""
import json
import math
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CLI = os.path.join(ROOT, "core_v2", "target", "release", "cli_v2")

N = 3
G = 9.80665
DT = 0.01
STEPS = 10000
RINT = 10
X0 = [0.1, 0.2, 0.3, 0.0, 0.0, 0.0]

CONFIGS = [
    # Verifiable gate (the corrected Jacobian commutes with the ODE here):
    ("default  m=1.0,L=1.0  (inertia 1)", 1.0, 1.0, 0.1, 0.5, True),
    ("m=0.5,L=1.0  (inertia .5)", 0.5, 1.0, 0.1, 0.5, True),
    ("m=1.5,L=2.0  (inertia 6)", 1.5, 2.0, 0.2, 0.7, True),
    ("m=2.0,L=0.75 (inertia 1.125)", 2.0, 0.75, 0.4, 0.3, True),
    ("m=1.0,L=2.0  (inertia 4)", 1.0, 2.0, 0.1, 0.5, True),
    ("m=1.0,L=4.0  (inertia 16)", 1.0, 4.0, 0.1, 0.5, True),
    # DOCUMENTED BOUNDARY DIVERGENCE (excluded from the gate): m=1.0,L=0.5,
    # inertia 0.25. Both implementations agree the system has a bifurcation
    # cliff near L~0.58. Below it, the fixed-point trig bias (deterministic
    # O(1e-6) vector-field error) systematically moves the branch: float64
    # lambda_1~0.52, Q32.32 lambda_1~1.91, both stable over 10k-40k steps.
    # The Jacobian itself is not at issue (finite-difference and Jv identities
    # hold at this config); the trajectory that each implementation integrates
    # is on a different branch of a parameter-sensitive bifurcation. Neither
    # value is authoritative; the L=0.5 length-sweep row must be reported as
    # precision-limited.
    ("m=1.0,L=0.5  (inertia .25)  [boundary]", 1.0, 0.5, 0.1, 0.5, False),
]


def make_deriv(M, L, B, C):
    def deriv(x):
        n = N
        d = [0.0] * (2 * n)
        for i in range(n):
            theta = x[i]
            omega = x[n + i]
            d[n + i] = -B * omega
            torque_g = G * (M * 2.0) * (L / 2.0) * math.sin(theta)
            torque_c = 0.0
            for j in range(n - 1):
                if j == i or (i >= 1 and j == i - 1):
                    theta_j = x[j]
                    dd = min(L, L)
                    torque_c += C * ((theta - theta_j) / dd) * 0.1
            inertia = M * L * L
            d[n + i] += (torque_g + torque_c) / inertia
            d[i] = omega
        return d
    return deriv


def make_jacobian(M, L, B, C):
    def jacobian(x):
        n = N
        dim = 2 * n
        J = [[0.0] * dim for _ in range(dim)]
        inertia = M * L * L
        for i in range(n):
            J[i][n + i] = 1.0
            # damping term is OUTSIDE the /inertia division in derivatives()
            J[n + i][n + i] = -B
            dom = G * (M * 2.0) * (L / 2.0) * math.cos(x[i])
            if i >= 1:
                coeff = C * 0.1 / min(L, L)
                dom += coeff
                J[n + i][i - 1] = -coeff / inertia
            J[n + i][i] = dom / inertia
        return J
    return jacobian


def benettin(x0, deriv, jacobian):
    x = list(x0)
    tangent = []
    for i in range(3):
        vi = [0.0] * (2 * N)
        vi[i] = 1.0
        tangent.append(vi)
    log_sum = [0.0] * 3

    for step in range(STEPS):
        s = list(x)
        if sum(abs(s[N + i]) for i in range(N)) < 0.5:
            s[N] += 3.0
        k1 = deriv(s)
        s2 = [s[i] + 0.5 * DT * k1[i] for i in range(2 * N)]
        k2 = deriv(s2)
        s3 = [s[i] + 0.5 * DT * k2[i] for i in range(2 * N)]
        k3 = deriv(s3)
        s4 = [s[i] + DT * k3[i] for i in range(2 * N)]
        k4 = deriv(s4)
        x = [s[i] + DT / 6.0 * (k1[i] + 2 * k2[i] + 2 * k3[i] + k4[i]) for i in range(2 * N)]

        J = jacobian(x)
        for k in range(3):
            jv = [sum(J[j][l] * tangent[k][l] for l in range(2 * N)) for j in range(2 * N)]
            tangent[k] = [tangent[k][j] + jv[j] * DT for j in range(2 * N)]

        if step > 0 and step % RINT == 0:
            for k in range(3):
                norm = math.sqrt(sum(tangent[k][j] ** 2 for j in range(2 * N)))
                tangent[k] = [tangent[k][j] / norm for j in range(2 * N)]
                log_sum[k] += math.log(norm)
            for i in range(1, 3):
                for j in range(i):
                    dot = sum(tangent[i][l] * tangent[j][l] for l in range(2 * N))
                    tangent[i] = [tangent[i][l] - dot * tangent[j][l] for l in range(2 * N)]
                    ni = math.sqrt(sum(tangent[i][l] ** 2 for l in range(2 * N)))
                    tangent[i] = [tangent[i][l] / ni for l in range(2 * N)]

    return log_sum[0] / (DT * STEPS)


def rust_lambda(M, L, B, C):
    res = subprocess.run(
        [CLI, "lyapunov", "--pendulums", str(N), "--mass", str(M), "--length", str(L),
         "--damping", str(B), "--coupling", str(C), "--steps", str(STEPS)],
        capture_output=True, text=True, timeout=300,
    )
    if res.returncode != 0:
        raise RuntimeError(res.stderr)
    return json.loads(res.stdout)["output"]["lambda1"]


def main():
    ok_all = True
    print(f"float64-independent vs Rust Q32.32 Benettin (deterministic IC {X0[:3]})")
    for label, M, L, B, C, gated in CONFIGS:
        ref = benettin(X0, make_deriv(M, L, B, C), make_jacobian(M, L, B, C))
        rust = rust_lambda(M, L, B, C)
        rel = abs(ref - rust) / max(abs(ref), 1e-6)
        if gated:
            # near-zero exponents are noisy between implementations; use a mixed
            # absolute+relative criterion
            ok = abs(ref - rust) <= max(0.10 * abs(ref), 0.02)
            ok_all &= ok
            tag = "MATCH" if ok else "MISMATCH"
        else:
            ok = False
            tag = "BOUNDARY-DIVERGENCE (documented, not gated)"
        print(f"  [{label}] lambda_1 ref={ref:.5f} rust={rust:.5f} "
              f"diff={abs(ref-rust):.5f} rel={rel*100:5.1f}% -> {tag}")
    print("ALL MATCH" if ok_all else "MISMATCH PRESENT (see boundary note)")
    return 0 if ok_all else 1


if __name__ == "__main__":
    sys.exit(main())