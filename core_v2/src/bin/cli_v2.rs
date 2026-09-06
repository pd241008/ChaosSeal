use clap::{Parser, Subcommand};
use chaosseal_core::*;
use serde::Serialize;
use rand::Rng;

#[derive(Parser)]
#[command(name = "chaosseal")]
#[command(about = "ChaosSeal protocol engine CLI (v2 with Attractor Sampling)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Lyapunov {
        #[arg(long, default_value = "3")]
        pendulums: usize,
        #[arg(long, default_value = "1.0")]
        mass: f64,
        #[arg(long, default_value = "1.0")]
        length: f64,
        #[arg(long, default_value = "0.1")]
        damping: f64,
        #[arg(long, default_value = "1.0")]
        coupling: f64,
        #[arg(long, default_value = "10000")]
        steps: usize,
    },
    LyapunovAttractor {
        #[arg(long, default_value = "3")]
        pendulums: usize,
        #[arg(long, default_value = "1.0")]
        mass: f64,
        #[arg(long, default_value = "1.0")]
        length: f64,
        #[arg(long, default_value = "0.1")]
        damping: f64,
        #[arg(long, default_value = "1.0")]
        coupling: f64,
        #[arg(long, default_value = "10000")]
        steps: usize,
        #[arg(long, default_value = "1000")]
        samples: usize,
    },
    /// Full top-3 Lyapunov spectrum per sampled initial condition, plus the
    /// Kolmogorov-Sinai entropy rate (sum of the positive exponents).
    LyapunovSpectrum {
        #[arg(long, default_value = "3")]
        pendulums: usize,
        #[arg(long, default_value = "1.0")]
        mass: f64,
        #[arg(long, default_value = "1.0")]
        length: f64,
        #[arg(long, default_value = "0.1")]
        damping: f64,
        #[arg(long, default_value = "1.0")]
        coupling: f64,
        #[arg(long, default_value = "10000")]
        steps: usize,
        #[arg(long, default_value = "1000")]
        samples: usize,
    },
    BeeSize {
        #[arg(long, default_value = "1024")]
        n: usize,
        #[arg(long, default_value = "8")]
        r: usize,
    },
    DeterminismTest,
}

#[derive(Serialize)]
struct ResultJson {
    success: bool,
    output: serde_json::Value,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Lyapunov { pendulums, mass, length, damping, coupling, steps } => {
            let pendulum = MultiPendulum::new(
                pendulums,
                Q32_32::from_f64(mass),
                Q32_32::from_f64(length),
                Q32_32::from_f64(damping),
                Q32_32::from_f64(coupling),
            );
            let mut state = vec![Q32_32::ZERO; pendulum.dimension()];
            for i in 0..pendulums {
                state[i] = Q32_32::from_f64(0.1 * (i as f64 + 1.0));
            }
            let estimator = LyapunovEstimator { steps, ..Default::default() };
            let lambda1 = estimator.estimate(&|t, s| pendulum.derivatives(t, s), &|s| pendulum.jacobian(s), &|s| pendulum.apply_reinjection(s), Q32_32::ZERO, &state);
            ResultJson {
                success: true,
                output: serde_json::json!({
                    "lambda1": lambda1.to_f64(),
                    "dt_bound": f64::max(256.0 * std::f64::consts::LN_2 / lambda1.to_f64(), 1.0 / lambda1.to_f64()),
                    "parameters": {
                        "pendulums": pendulums,
                        "mass": mass,
                        "length": length,
                        "damping": damping,
                        "coupling": coupling,
                        "steps": steps,
                    }
                }),
            }
        }
        Commands::LyapunovAttractor { pendulums, mass, length, damping, coupling, steps, samples } => {
            let pendulum = MultiPendulum::new(
                pendulums,
                Q32_32::from_f64(mass),
                Q32_32::from_f64(length),
                Q32_32::from_f64(damping),
                Q32_32::from_f64(coupling),
            );
            
            let mut rng = rand::thread_rng();
            let mut lambda1s = Vec::new();
            let mut initial_thetas = Vec::new();
            
            for _ in 0..samples {
                let mut state = vec![Q32_32::ZERO; pendulum.dimension()];
                for i in 0..pendulum.dimension() {
                    // Sample between -PI and PI roughly
                    let val = rng.gen_range(-3.14159..3.14159);
                    state[i] = Q32_32::from_f64(val);
                }
                
                let estimator = LyapunovEstimator { steps, ..Default::default() };
                let lambda1 = estimator.estimate(&|t, s| pendulum.derivatives(t, s), &|s| pendulum.jacobian(s), &|s| pendulum.apply_reinjection(s), Q32_32::ZERO, &state);
                lambda1s.push(lambda1.to_f64());
                initial_thetas.push(state[0].to_f64());
            }
            
            let min = lambda1s.iter().copied().fold(f64::INFINITY, f64::min);
            let max = lambda1s.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let sum: f64 = lambda1s.iter().sum();
            let mean = sum / (samples as f64);
            
            // max dt bound is determined by the max lambda1
            let max_dt_bound = f64::max(256.0 * std::f64::consts::LN_2 / max, 1.0 / max);
            let min_dt_bound = f64::max(256.0 * std::f64::consts::LN_2 / min, 1.0 / min);

            ResultJson {
                success: true,
                output: serde_json::json!({
                    "samples": samples,
                    "lambda1_min": min,
                    "lambda1_max": max,
                    "lambda1_mean": mean,
                    "dt_bound_min": min_dt_bound,
                    "dt_bound_max": max_dt_bound,
                    "raw_lambda1": lambda1s,
                    "initial_thetas": initial_thetas,
                }),
            }
        }
        Commands::LyapunovSpectrum { pendulums, mass, length, damping, coupling, steps, samples } => {
            let pendulum = MultiPendulum::new(
                pendulums,
                Q32_32::from_f64(mass),
                Q32_32::from_f64(length),
                Q32_32::from_f64(damping),
                Q32_32::from_f64(coupling),
            );

            let mut rng = rand::thread_rng();
            let mut l1s: Vec<f64> = Vec::new();
            let mut l2s: Vec<f64> = Vec::new();
            let mut l3s: Vec<f64> = Vec::new();
            let mut ks: Vec<f64> = Vec::new();
            let mut spectra: Vec<Vec<f64>> = Vec::new();

            for _ in 0..samples {
                let mut state = vec![Q32_32::ZERO; pendulum.dimension()];
                for i in 0..pendulum.dimension() {
                    let val = rng.gen_range(-3.14159..3.14159);
                    state[i] = Q32_32::from_f64(val);
                }
                let estimator = LyapunovEstimator { steps, ..Default::default() };
                let spec = estimator.estimate_spectrum(
                    &|t, s| pendulum.derivatives(t, s),
                    &|s| pendulum.jacobian(s),
                    &|s| pendulum.apply_reinjection(s),
                    Q32_32::ZERO, &state);
                let s: Vec<f64> = spec.iter().map(|x| x.to_f64()).collect();
                l1s.push(s[0]); l2s.push(s[1]); l3s.push(s[2]);
                ks.push(s.iter().filter(|&x| *x > 0.0).sum());
                spectra.push(s);
            }

            let stats = |v: &[f64]| (v.iter().copied().fold(f64::INFINITY, f64::min),
                                    v.iter().sum::<f64>() / v.len() as f64,
                                    v.iter().copied().fold(f64::NEG_INFINITY, f64::max));
            let (l1_min, l1_mean, _) = stats(&l1s);
            let (_, l2_mean, l2_max) = stats(&l2s);
            let (_, l3_mean, l3_max) = stats(&l3s);
            let (ks_min, ks_mean, ks_max) = stats(&ks);
            let m = |x: f64| f64::max(256.0 * std::f64::consts::LN_2 / x, 1.0 / x);

            ResultJson {
                success: true,
                output: serde_json::json!({
                    "samples": samples,
                    "lambda1_min": l1_min,
                    "lambda1_mean": l1_mean,
                    "lambda2_mean": l2_mean,
                    "lambda2_max": l2_max,
                    "lambda3_mean": l3_mean,
                    "lambda3_max": l3_max,
                    "ks_mean": ks_mean,
                    "ks_min": ks_min,
                    "ks_max": ks_max,
                    "l2_positive_samples": l2s.iter().filter(|&x| *x > 0.0).count(),
                    "l3_positive_samples": l3s.iter().filter(|&x| *x > 0.0).count(),
                    "dt_bound_from_l1_min_s": m(l1_min),
                    "dt_bound_from_ks_min_s": m(ks_min),
                    "raw_spectra": spectra,
                }),
            }
        }
        Commands::BeeSize { n, r } => {
            let engine = BEEEngine::new(n, r);
            ResultJson {
                success: true,
                output: serde_json::json!({
                    "n": n,
                    "r": r,
                    "ciphertext_size_bytes": engine.ciphertext_size_min(),
                }),
            }
        }
        Commands::DeterminismTest => {
            let mut results = Vec::new();
            for _ in 0..10 {
                let engine = BEEEngine::new(1024, 8);
                results.push(engine.ciphertext_size_min());
            }
            let deterministic = results.windows(2).all(|w| w[0] == w[1]);
            ResultJson {
                success: deterministic,
                output: serde_json::json!({
                    "deterministic": deterministic,
                    "sizes": results,
                }),
            }
        }
    };

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    if !result.success {
        std::process::exit(1);
    }
}
