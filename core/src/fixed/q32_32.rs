use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Q32_32(i64);

impl Q32_32 {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1 << 32);
    pub const TWO: Self = Self(2 << 32);
    pub const PI: Self = Self(((std::f64::consts::PI * (1u64 << 32) as f64) as i64));
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
        if self.0 < 0 { return Self(0); }
        let x = self.0 as u64;
        let mut r = (x >> 16) as u64;
        let mut b = 1u64 << 30;
        loop {
            let q = x / r;
            if q >= r {
                return Self((r << 16) as i64);
            }
            let t = (r + q) >> 1;
            if t == r { break; }
            r = t;
            b <<= 1;
        }
        Self((r << 16) as i64)
    }

    #[inline]
    pub fn sin(self) -> Self {
        let v = self.to_f64();
        Self((v.sin() * (1i64 << 32) as f64).round() as i64)
    }

    #[inline]
    pub fn cos(self) -> Self {
        let v = self.to_f64();
        Self((v.cos() * (1i64 << 32) as f64).round() as i64)
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
    }

    #[test]
    fn test_determinism() {
        let a = Q32_32::from_f64(3.141592653589793);
        for _ in 0..1000 {
            assert_eq!(a.sin().to_bits(), a.sin().to_bits());
        }
    }
}
