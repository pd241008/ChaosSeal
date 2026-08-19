use chaosseal_core::{MultiPendulum, Rk4Integrator, Q32_32};
use std::time::Instant;

fn main() {
    let pendulum = MultiPendulum::new(3, Q32_32::from_f64(1.0), Q32_32::from_f64(1.0), Q32_32::from_f64(0.0), Q32_32::from_f64(0.5));
    let mut state = vec![Q32_32::ZERO; pendulum.dimension()];
    for i in 0..3 { state[i] = Q32_32::from_f64(3.0); }
    let integrator = Rk4Integrator::new(Q32_32::from_f64(0.1));
    let start = Instant::now();
    let (_, _) = integrator.integrate(&|t, s| pendulum.derivatives(t, s), Q32_32::ZERO, &state, 12000);
    println!("Time for 12000 steps: {:?}", start.elapsed());
}
