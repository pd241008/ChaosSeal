use crate::fixed::Q32_32;

#[derive(Clone, Debug)]
pub struct LyapunovEstimator {
    pub dt: Q32_32,
    pub steps: usize,
    pub reorthonormalize_interval: usize,
    pub tangent_dim: usize,
}

impl Default for LyapunovEstimator {
    fn default() -> Self {
        Self {
            dt: Q32_32::from_f64(0.01),
            steps: 10000,
            reorthonormalize_interval: 50,
            tangent_dim: 3,
        }
    }
}

impl LyapunovEstimator {
    pub fn estimate<F, G>(&self, system: &F, jacobian: &G, t0: Q32_32, initial_state: &[Q32_32]) -> Q32_32
    where
        F: Fn(Q32_32, &[Q32_32]) -> Vec<Q32_32>,
        G: Fn(&[Q32_32]) -> Vec<Vec<Q32_32>>,
    {
        let mut traj = initial_state.to_vec();
        let state_dim = initial_state.len();
        assert!(self.tangent_dim <= state_dim, "tangent_dim must not exceed the phase-space dimension");
        let mut tangent = vec![vec![Q32_32::ZERO; state_dim]; self.tangent_dim];

        for i in 0..self.tangent_dim {
            tangent[i][i] = Q32_32::ONE;
        }

        let mut log_sum = vec![Q32_32::ZERO; self.tangent_dim];

        for step in 0..self.steps {
            let (_nt, ns) = {
                let dt = self.dt;
                let mut t = t0 + Q32_32::from_f64(step as f64) * dt;
                let mut s = traj.clone();
                for _ in 0..1 {
                    let k1 = system(t, &s);
                    let t2 = t + dt / Q32_32::from_f64(2.0);
                    let mut s2 = vec![Q32_32::ZERO; s.len()];
                    for i in 0..s.len() { s2[i] = s[i] + (k1[i] * dt) / Q32_32::from_f64(2.0); }
                    let k2 = system(t2, &s2);
                    let mut s3 = vec![Q32_32::ZERO; s.len()];
                    for i in 0..s.len() { s3[i] = s[i] + (k2[i] * dt) / Q32_32::from_f64(2.0); }
                    let k3 = system(t2, &s3);
                    let t4 = t + dt;
                    let mut s4 = vec![Q32_32::ZERO; s.len()];
                    for i in 0..s.len() { s4[i] = s[i] + k3[i] * dt; }
                    let k4 = system(t4, &s4);
                    for i in 0..s.len() {
                        let sum = k1[i] + k2[i] * Q32_32::from_f64(2.0) + k3[i] * Q32_32::from_f64(2.0) + k4[i];
                        s[i] = s[i] + (sum * dt) / Q32_32::from_f64(6.0);
                    }
                    t = t4;
                }
                (t, s)
            };

            traj = ns;

            let jac = jacobian(&traj);
            for i in 0..self.tangent_dim {
                let tv = tangent[i].clone();
                let mut jv = vec![Q32_32::ZERO; state_dim];
                for j in 0..state_dim {
                    let mut acc = Q32_32::ZERO;
                    for k in 0..state_dim {
                        acc = acc + jac[j][k] * tv[k];
                    }
                    jv[j] = acc;
                }
                let mut new_tv = vec![Q32_32::ZERO; state_dim];
                for j in 0..state_dim {
                    new_tv[j] = tv[j] + jv[j] * self.dt;
                }
                tangent[i] = new_tv;
            }

            if step > 0 && step % self.reorthonormalize_interval == 0 {
                let mut norms = vec![Q32_32::ZERO; self.tangent_dim];
                for i in 0..self.tangent_dim {
                    for j in 0..tangent[i].len() {
                        norms[i] = norms[i] + tangent[i][j] * tangent[i][j];
                    }
                    norms[i] = norms[i].sqrt();
                    if norms[i] != Q32_32::ZERO {
                        for j in 0..tangent[i].len() {
                            tangent[i][j] = tangent[i][j] / norms[i];
                        }
                    }
                    log_sum[i] = log_sum[i] + norms[i].ln();
                }

                for i in 1..self.tangent_dim {
                    for j in 0..i {
                        let mut dot = Q32_32::ZERO;
                        for k in 0..tangent[i].len() {
                            dot = dot + tangent[i][k] * tangent[j][k];
                        }
                        for k in 0..tangent[i].len() {
                            tangent[i][k] = tangent[i][k] - dot * tangent[j][k];
                        }
                        let mut norm_i = Q32_32::ZERO;
                        for k in 0..tangent[i].len() {
                            norm_i = norm_i + tangent[i][k] * tangent[i][k];
                        }
                        norm_i = norm_i.sqrt();
                        if norm_i != Q32_32::ZERO {
                            for k in 0..tangent[i].len() {
                                tangent[i][k] = tangent[i][k] / norm_i;
                            }
                        }
                }
            }
        }
        }

        let total_t = Q32_32::from_f64(self.steps as f64) * self.dt;
        let lambda1 = log_sum[0] / total_t;
        lambda1
    }
}
