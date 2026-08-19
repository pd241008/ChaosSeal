import subprocess
import json
import sys
import csv

def main():
    samples = 1000
    if len(sys.argv) > 1:
        samples = int(sys.argv[1])
        
    print(f"Running ChaosSeal v2 Lyapunov Attractor Sampling ({samples} samples)...")
    res = subprocess.run(
        ["cargo", "run", "--release", "--bin", "cli_v2", "--", "lyapunov-attractor", "--samples", str(samples)],
        cwd="core_v2", capture_output=True, text=True
    )
    
    if res.returncode == 0:
        data = json.loads(res.stdout)
        out = data['output']
        
        # Write raw samples to CSV
        raw_lambdas = out.get('raw_lambda1', [])
        initial_thetas = out.get('initial_thetas', [])
        if raw_lambdas and len(raw_lambdas) == len(initial_thetas):
            csv_path = "results_v2/lyapunov_samples.csv"
            with open(csv_path, 'w', newline='') as f:
                writer = csv.writer(f)
                writer.writerow(["Sample_ID", "Initial_Theta1_rad", "Lambda1_nats_per_s"])
                for i, (val, theta) in enumerate(zip(raw_lambdas, initial_thetas)):
                    writer.writerow([i+1, theta, val])
            print(f"Raw samples exported to {csv_path}")

        print("\nLyapunov Attractor Sampling Results:")
        print("="*40)
        print(f"Samples: {out['samples']}")
        print(f"Min Lambda1: {out['lambda1_min']:.4f} nats/s")
        print(f"Max Lambda1: {out['lambda1_max']:.4f} nats/s")
        print(f"Mean Lambda1: {out['lambda1_mean']:.4f} nats/s")
        print(f"Min DT Bound (s): {out['dt_bound_min']:.1f} s")
        print(f"Max DT Bound (s): {out['dt_bound_max']:.1f} s")
        print("="*40)
        print("Conclusion: The max lambda1 defines the minimum dt bound.")
        if out['dt_bound_min'] >= 840.6:
            print("Verified: DT Bound holds above 840.6s across the attractor.")
        else:
            print(f"Warning: DT Bound dropped below 840.6s! (minimum was {out['dt_bound_min']:.1f})")
    else:
        print("Error running sampling:")
        print(res.stderr)

if __name__ == "__main__":
    main()
