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
        #[arg(long, default_value = "0.5")]
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
        #[arg(long, default_value = "0.5")]
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
            let lambda1 = estimator.estimate(&|t, s| pendulum.derivatives(t, s), Q32_32::ZERO, &state);
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
            
            for _ in 0..samples {
                let mut state = vec![Q32_32::ZERO; pendulum.dimension()];
                for i in 0..pendulum.dimension() {
                    // Sample between -PI and PI roughly
                    let val = rng.gen_range(-3.14159..3.14159);
                    state[i] = Q32_32::from_f64(val);
                }
                
                let estimator = LyapunovEstimator { steps, ..Default::default() };
                let lambda1 = estimator.estimate(&|t, s| pendulum.derivatives(t, s), Q32_32::ZERO, &state);
                lambda1s.push(lambda1.to_f64());
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
