use crate::fixed::Q32_32;

#[derive(Clone, Debug)]
pub struct MultiPendulum {
    pub masses: Vec<Q32_32>,
    pub lengths: Vec<Q32_32>,
    pub damping: Vec<Q32_32>,
    pub couplings: Vec<Q32_32>,
}

impl MultiPendulum {
    pub fn new(n: usize, mass: Q32_32, length: Q32_32, damping: Q32_32, coupling: Q32_32) -> Self {
        Self {
            masses: vec![mass; n],
            lengths: vec![length; n],
            damping: vec![damping; n],
            couplings: vec![coupling; n.saturating_sub(1)],
        }
    }

    pub fn dimension(&self) -> usize {
        self.masses.len() * 2
    }

    pub fn gravity() -> Q32_32 {
        Q32_32::from_f64(9.80665)
    }

    /// Deterministic energy re-injection using purely Q32.32 arithmetic.
    /// If the sum of absolute angular velocities falls below a threshold,
    /// an impulse is added to the first mass's velocity.
    pub fn apply_reinjection(&self, state: &mut [Q32_32]) {
        let n = self.masses.len();
        let omega_thresh = Q32_32::from_bits(1 << 31); // 0.5 in Q32.32
        let omega_boost = Q32_32::from_bits(3 << 32);  // 3.0 in Q32.32

        let mut sum_omega = Q32_32::ZERO;
        for i in 0..n {
            sum_omega = sum_omega + state[n + i].abs();
        }

        if sum_omega < omega_thresh {
            state[n] = state[n] + omega_boost;
        }
    }

    pub fn derivatives(&self, _t: Q32_32, state: &[Q32_32]) -> Vec<Q32_32> {
        let n = self.masses.len();
        let mut deriv = vec![Q32_32::ZERO; self.dimension()];

        let g = Self::gravity();

        for i in 0..n {
            let theta_i = state[i];
            let omega_i = state[n + i];
            deriv[n + i] = -self.damping[i] * omega_i;

            let torque_g = g * (self.masses[i] * Q32_32::from_f64(2.0)) * (self.lengths[i] / Q32_32::from_f64(2.0)) * theta_i.sin();
            let mut torque_c = Q32_32::ZERO;

            for (j, coupling) in self.couplings.iter().enumerate() {
                if j == i || (i >= 1 && j == i - 1) {
                    let theta_j = state[j];
                    let d = self.lengths[i].min(self.lengths[j]);
                    torque_c = torque_c + *coupling * ((theta_i - theta_j) / d) * Q32_32::from_f64(0.1);
                }
            }

            let inertia = self.masses[i] * self.lengths[i] * self.lengths[i];
            if inertia != Q32_32::ZERO {
                deriv[n + i] = deriv[n + i] + (torque_g + torque_c) / inertia;
            }

            deriv[i] = omega_i;
        }

        deriv
    }
}
