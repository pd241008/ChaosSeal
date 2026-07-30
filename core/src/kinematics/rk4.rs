use crate::fixed::Q32_32;

#[derive(Clone, Debug)]
pub struct Rk4Integrator {
    pub dt: Q32_32,
}

impl Rk4Integrator {
    pub fn new(dt: Q32_32) -> Self { Self { dt } }

    pub fn step<F>(&self, system: &F, t: Q32_32, state: &[Q32_32]) -> (Q32_32, Vec<Q32_32>)
    where
        F: Fn(Q32_32, &[Q32_32]) -> Vec<Q32_32>,
    {
        let k1 = system(t, state);
        let t2 = t + self.dt / Q32_32::from_f64(2.0);
        let mut s2 = vec![Q32_32::ZERO; state.len()];
        for i in 0..state.len() {
            s2[i] = state[i] + (k1[i] * self.dt) / Q32_32::from_f64(2.0);
        }
        let k2 = system(t2, &s2);
        let mut s3 = vec![Q32_32::ZERO; state.len()];
        for i in 0..state.len() {
            s3[i] = state[i] + (k2[i] * self.dt) / Q32_32::from_f64(2.0);
        }
        let k3 = system(t2, &s3);
        let t4 = t + self.dt;
        let mut s4 = vec![Q32_32::ZERO; state.len()];
        for i in 0..state.len() {
            s4[i] = state[i] + k3[i] * self.dt;
        }
        let k4 = system(t4, &s4);

        let mut new_state = vec![Q32_32::ZERO; state.len()];
        for i in 0..state.len() {
            let sum = k1[i] + k2[i] * Q32_32::from_f64(2.0) + k3[i] * Q32_32::from_f64(2.0) + k4[i];
            new_state[i] = state[i] + (sum * self.dt) / Q32_32::from_f64(6.0);
        }

        (t + self.dt, new_state)
    }

    pub fn integrate<F>(&self, system: &F, t0: Q32_32, state: &[Q32_32], steps: usize) -> (Q32_32, Vec<Q32_32>)
    where
        F: Fn(Q32_32, &[Q32_32]) -> Vec<Q32_32>,
    {
        let mut t = t0;
        let mut s = state.to_vec();
        for _ in 0..steps {
            let (nt, ns) = self.step(system, t, &s);
            t = nt;
            s = ns;
        }
        (t, s)
    }
}
