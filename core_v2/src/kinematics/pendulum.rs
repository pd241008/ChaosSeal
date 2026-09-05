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

    /// Jacobian of `derivatives` with respect to the state
    /// `x = [theta_0..theta_{n-1}, omega_0..omega_{n-1}]`.
    ///
    /// Matches the exact arithmetic of `derivatives` (including the coupling
    /// convention: only the left neighbor `i-1` contributes a non-zero term,
    /// since the `j == i` branch is the identically-zero self term
    /// `(theta_i - theta_i)`). Row `n + i` is
    /// `d(omega_i')/dx` scaled by `1/inertia`.
    pub fn jacobian(&self, state: &[Q32_32]) -> Vec<Vec<Q32_32>> {
        let n = self.masses.len();
        let dim = self.dimension();
        let mut jac = vec![vec![Q32_32::ZERO; dim]; dim];
        let g = Self::gravity();
        let two = Q32_32::from_f64(2.0);
        let half = Q32_32::from_f64(0.5);
        let tenth = Q32_32::from_f64(0.1);

        for i in 0..n {
            // theta_i' = omega_i  =>  d(theta_i')/d(omega_i) = 1
            jac[i][n + i] = Q32_32::ONE;

            let inertia = self.masses[i] * self.lengths[i] * self.lengths[i];
            if inertia == Q32_32::ZERO {
                continue;
            }

            // d(omega_i')/d(omega_i) = -damping[i] / inertia
            jac[n + i][n + i] = -self.damping[i] / inertia;

            // d(omega_i')/d(theta_i):
            //   torque_g  = g * (m_i*2) * (L_i/2) * sin(theta_i)
            //   => derivative = g * (m_i*2) * (L_i/2) * cos(theta_i)
            let m2 = self.masses[i] * two;
            let lh = self.lengths[i] * half;
            let mut d_om = g * m2 * lh * state[i].cos();

            // coupling tau_c = c_{i-1} * ((theta_i - theta_{i-1}) / d) * 0.1
            // for i >= 1; d(tau_c)/d(theta_i) = +c*0.1/d,
            // d(tau_c)/d(theta_{i-1}) = -c*0.1/d.
            if i >= 1 {
                let coupling = self.couplings[i - 1];
                let d = self.lengths[i].min(self.lengths[i - 1]);
                let coeff = coupling * tenth / d;
                d_om = d_om + coeff;
                jac[n + i][i - 1] = -coeff;
            }

            jac[n + i][i] = d_om / inertia;
        }

        jac
    }
}
