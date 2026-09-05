use clap::{Parser, Subcommand};
use chaosseal_core::*;
use serde::Serialize;

#[derive(Parser)]
#[command(name = "chaosseal")]
#[command(about = "ChaosSeal protocol engine CLI")]
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
            let lambda1 = estimator.estimate(&|t, s| pendulum.derivatives(t, s), &|s| pendulum.jacobian(s), Q32_32::ZERO, &state);
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
