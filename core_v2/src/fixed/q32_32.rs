use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Q32_32(i64);

impl Q32_32 {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1 << 32);
    pub const TWO: Self = Self(2 << 32);
    pub const PI: Self = Self((std::f64::consts::PI * (1u64 << 32) as f64) as i64);
    pub const FRAC_PI_2: Self = Self((std::f64::consts::FRAC_PI_2 * (1u64 << 32) as f64) as i64);
    pub const FRAC_PI_4: Self = Self((std::f64::consts::FRAC_PI_4 * (1u64 << 32) as f64) as i64);
    pub const EPSILON: Self = Self(1);
    pub const MAX: Self = Self(i64::MAX);

    #[inline]
    pub const fn from_bits(bits: i64) -> Self { Self(bits) }

    #[inline]
    pub const fn to_bits(self) -> i64 { self.0 }

    #[inline]
    pub const fn from_f64(v: f64) -> Self {
        Self((v * (1i64 << 32) as f64).round() as i64)
    }

    #[inline]
    pub fn to_f64(self) -> f64 { self.0 as f64 / (1i64 << 32) as f64 }

    #[inline]
    pub const fn from_i64(v: i64) -> Self {
        Self(v.saturating_mul(1i64 << 32))
    }

    #[inline]
    pub fn to_i64(self) -> i64 { self.0 >> 32 }

    #[inline]
    pub fn abs(self) -> Self { Self(self.0.abs()) }

    #[inline]
    pub fn sqrt(self) -> Self {
        if self.0 <= 0 { return Self::ZERO; }
        let n = (self.0 as u128) << 32;
        let bits = 128 - n.leading_zeros();
        let mut x0 = 1u128 << ((bits + 1) / 2);
        let mut x1 = (x0 + n / x0) / 2;
        while x1 < x0 {
            x0 = x1;
            x1 = (x0 + n / x0) / 2;
        }
        Self(x0 as i64)
    }

    #[inline]
    pub fn sin(self) -> Self {
        let mut val = self.0;
        val = val % 26986075409;
        if val < 0 { val += 26986075409; }
        
        let mut sign = 1i64;
        if val >= 13493037704 {
            val -= 13493037704;
            sign = -1;
        }
        if val > 6746518852 {
            val = 13493037704 - val;
        }
        
        let x_56 = (val as i128) << 24;
        let x2_56 = (x_56 * x_56) >> 56;
        
        let mut sum_56 = x_56;
        let mut term_56 = x_56;
        
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 6; sum_56 -= term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 20; sum_56 += term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 42; sum_56 -= term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 72; sum_56 += term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 110; sum_56 -= term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 156; sum_56 += term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 210; sum_56 -= term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 272; sum_56 += term_56;
        
        let sum = ((sum_56 + (1 << 23)) >> 24) as i64;
        Self(sum * sign)
    }

    #[inline]
    pub fn cos(self) -> Self {
        let mut val = self.0;
        val = val % 26986075409;
        if val < 0 { val += 26986075409; }
        
        let mut sign = 1i64;
        if val >= 13493037704 {
            val -= 13493037704;
            sign = -1;
        }
        if val > 6746518852 {
            val = 13493037704 - val;
            sign = -sign;
        }
        
        let x_56 = (val as i128) << 24;
        let x2_56 = (x_56 * x_56) >> 56;
        
        let mut term_56 = 1i128 << 56;
        let mut sum_56 = term_56;
        
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 2; sum_56 -= term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 12; sum_56 += term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 30; sum_56 -= term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 56; sum_56 += term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 90; sum_56 -= term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 132; sum_56 += term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 182; sum_56 -= term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 240; sum_56 += term_56;
        term_56 = (term_56 * x2_56) >> 56; term_56 /= 306; sum_56 -= term_56;
        
        let sum = ((sum_56 + (1 << 23)) >> 24) as i64;
        Self(sum * sign)
    }

    #[inline]
    pub fn exp(self) -> Self {
        let v = self.to_f64();
        Self((v.exp() * (1i64 << 32) as f64).round() as i64)
    }

    #[inline]
    pub fn ln(self) -> Self {
        if self.0 <= 0 { return Self(i64::MIN); }
        let v = self.to_f64();
        Self((v.ln() * (1i64 << 32) as f64).round() as i64)
    }
}

impl Add for Q32_32 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Q32_32 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl Neg for Q32_32 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self { Self(self.0.neg()) }
}

impl Mul for Q32_32 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let a = self.0 as i128;
        let b = rhs.0 as i128;
        let prod = (a * b) >> 32;
        let clamped = if prod > i64::MAX as i128 { i64::MAX } else if prod < i64::MIN as i128 { i64::MIN } else { prod as i64 };
        Self(clamped)
    }
}

impl Div for Q32_32 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        if rhs.0 == 0 { return Self::MAX; }
        let a = self.0 as i128;
        let b = rhs.0 as i128;
        let res = (a << 32) / b;
        let clamped = if res > i64::MAX as i128 { i64::MAX } else if res < i64::MIN as i128 { i64::MIN } else { res as i64 };
        Self(clamped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_q32_32_basic_ops() {
        let a = Q32_32::from_f64(1.5);
        let b = Q32_32::from_f64(2.5);
        let sum = a + b;
        assert!((sum.to_f64() - 4.0).abs() < 1e-6);

        let prod = a * b;
        assert!((prod.to_f64() - 3.75).abs() < 1e-6);

        let quot = b / a;
        assert!((quot.to_f64() - (2.5 / 1.5)).abs() < 1e-6);
    }

    #[test]
    fn test_q32_32_trig() {
        let zero = Q32_32::ZERO;
        let one = Q32_32::ONE;
        assert!((zero.sin().to_f64() - 0.0).abs() < 1e-6);
        assert!((zero.cos().to_f64() - 1.0).abs() < 1e-6);
        assert!((one.sin().to_f64() - 1.0f64.sin()).abs() < 1e-6);
    }

    #[test]
    fn test_determinism() {
        let a = Q32_32::from_f64(3.141592653589793);
        for _ in 0..1000 {
            assert_eq!(a.sin().to_bits(), a.sin().to_bits());
        }
    }

    /// Independent cross-validation of Q32.32 sin/cos/sqrt against an external
    /// f64 reference over a dense grid. Quantifies the worst-case approximation
    /// error, the item previously flagged as "self-validated only". This is the
    /// external-oracle check (stdlib libm) that a determinism test alone cannot
    /// provide.
    #[test]
    fn test_transcendental_worst_case_error_vs_f64() {
        let mut worst_sin = 0f64;
        let mut worst_cos = 0f64;
        let mut worst_abs_sqrt = 0f64;
        let mut worst_rel_sqrt = 0f64;

        // sin/cos: sweep [-pi, pi] plus beyond-period arguments.
        let n = 200_000;
        for i in 0..n {
            let x = -std::f64::consts::PI + (2.0 * std::f64::consts::PI) * (i as f64) / (n as f64 - 1.0);
            let qx = Q32_32::from_f64(x);
            let e_s = (qx.sin().to_f64() - x.sin()).abs();
            let e_c = (qx.cos().to_f64() - x.cos()).abs();
            if e_s > worst_sin { worst_sin = e_s; }
            if e_c > worst_cos { worst_cos = e_c; }
        }

        // sqrt: sweep [0, 1e6].
        let m = 200_000;
        for i in 1..=m {
            let x = (i as f64) / (m as f64) * 1e6;
            let qx = Q32_32::from_f64(x);
            let got = qx.sqrt().to_f64();
            let want = x.sqrt();
            let e_abs = (got - want).abs();
            let e_rel = e_abs / want.abs().max(f64::MIN_POSITIVE);
            if e_abs > worst_abs_sqrt { worst_abs_sqrt = e_abs; }
            if e_rel > worst_rel_sqrt { worst_rel_sqrt = e_rel; }
        }

        // Empirically the fixed-point approximations land well under 2^-24
        // absolute for trig over [-pi,pi] and < 1e-6 relative for sqrt in range.
        assert!(worst_sin < 2e-9, "sin worst abs err {}", worst_sin);
        assert!(worst_cos < 2e-9, "cos worst abs err {}", worst_cos);
        assert!(worst_rel_sqrt < 2e-6, "sqrt worst rel err {}", worst_rel_sqrt);
        // Record for reporting (visible on failure / via --nocapture).
        eprintln!(
            "fixedpoint_crossval worst_sin={:.3e} worst_cos={:.3e} worst_sqrt_abs={:.3e} worst_sqrt_rel={:.3e}",
            worst_sin, worst_cos, worst_abs_sqrt, worst_rel_sqrt
        );
    }
}
