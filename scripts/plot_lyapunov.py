import matplotlib.pyplot as plt
import csv

theta1s = []
lambdas = []

with open("results_v2/lyapunov_samples.csv", "r") as f:
    reader = csv.DictReader(f)
    for row in reader:
        theta1s.append(float(row["Initial_Theta1_rad"]))
        lambdas.append(float(row["Lambda1_nats_per_s"]))

plt.figure(figsize=(10, 6))
plt.scatter(theta1s, lambdas, alpha=0.6, edgecolors='k')
plt.xlabel("Initial Theta_1 (rad)")
plt.ylabel("Lambda_1 (nats/s)")
plt.title("Lyapunov Exponent vs Initial Angle (Energy Regimes)")
plt.grid(True)
plt.tight_layout()
plt.savefig("results_v2/lyapunov_bimodal_plot.png")
print("Saved plot to results_v2/lyapunov_bimodal_plot.png")
