#!/usr/bin/env python3
"""Independent float64 reference of the Benettin estimator with the corrected
Jacobian tangent update (v <- v + J(x)v*dt), cross-checked against the Rust
Q32.32 fixed-point implementation.

Run:  python3 scripts/validate_benettin.py
It replicates core_v2/lyapunov/benettin.rs + kinematics/pendulum.rs exactly in
float64 (same ODE, same coupling convention, same reinjection, same RK4 step,
same reorthonormalization schedule) and compares the resulting lambda_1 for the
deterministic initial condition [0.1, 0.2, 0.3, 0, 0, 0] (3 pendulums,
m=1.0, L=1.0, b=0.1, c=0.5).
"""
import json
import math
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CLI = os.path.join(ROOT, "core_v2", "target", "release", "cli_v2")

N = 3
M = 1.0
L = 1.0
B = 0.1
C = 0.5
G = 9.80665
DT = 0.01
STEPS = 10000
RINT = 10
X0 = [0.1, 0.2, 0.3, 0.0, 0.0, 0.0]


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


def jacobian(x):
    n = N
    dim = 2 * n
    J = [[0.0] * dim for _ in range(dim)]
    inertia = M * L * L
    for i in range(n):
        J[i][n + i] = 1.0
        J[n + i][n + i] = -B / inertia
        dom = G * (M * 2.0) * (L / 2.0) * math.cos(x[i])
        if i >= 1:
            coeff = C * 0.1 / min(L, L)
            dom += coeff
            J[n + i][i - 1] = -coeff
        J[n + i][i] = dom / inertia
    return J


def benettin(x0):
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


def rust_lambda():
    res = subprocess.run(
        [CLI, "lyapunov", "--pendulums", str(N), "--mass", str(M), "--length", str(L),
         "--damping", str(B), "--coupling", str(C), "--steps", str(STEPS)],
        capture_output=True, text=True, timeout=300,
    )
    if res.returncode != 0:
        raise RuntimeError(res.stderr)
    return json.loads(res.stdout)["output"]["lambda1"]


def main():
    ref = benettin(X0)
    rust = rust_lambda()
    print(f"float64 reference lambda_1 : {ref:.6f}")
    print(f"Rust Q32.32 lambda_1        : {rust:.6f}")
    print(f"relative difference         : {abs(ref - rust) / abs(ref) * 100:.2f}%")
    ok = abs(ref - rust) / abs(ref) < 0.15
    print("MATCH" if ok else "MISMATCH")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())