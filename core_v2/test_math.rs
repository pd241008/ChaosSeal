use std::f64::consts::PI;

const PI_Q32: i64 = 13493037704;
const PI_2_Q32: i64 = 26986075409;
const PI_HALF_Q32: i64 = 6746518852;

fn sin_q32(mut val: i64) -> i64 {
    val = val % PI_2_Q32;
    if val < 0 { val += PI_2_Q32; }
    
    let mut sign = 1i64;
    if val >= PI_Q32 {
        val -= PI_Q32;
        sign = -1;
    }
    if val > PI_HALF_Q32 {
        val = PI_Q32 - val;
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
    sum * sign
}

fn cos_q32(mut val: i64) -> i64 {
    val = val % PI_2_Q32;
    if val < 0 { val += PI_2_Q32; }
    
    let mut sign = 1i64;
    if val >= PI_Q32 {
        val -= PI_Q32;
        sign = -1;
    }
    if val > PI_HALF_Q32 {
        val = PI_Q32 - val;
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
    sum * sign
}

fn sqrt_q32(val: i64) -> i64 {
    if val <= 0 { return 0; }
    let n = (val as u128) << 32;
    let bits = 128 - n.leading_zeros();
    let mut x0 = 1u128 << ((bits + 1) / 2);
    let mut x1 = (x0 + n / x0) / 2;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + n / x0) / 2;
    }
    x0 as i64
}

fn main() {
    let mut max_err_sin = 0f64;
    let mut max_err_cos = 0f64;
    let mut max_err_sqrt = 0f64;
    
    let scale = (1i64 << 32) as f64;
    
    // Test sin/cos
    for i in -1000..=1000 {
        let x = (i as f64) * 0.01;
        let x_q32 = (x * scale).round() as i64;
        
        let sin_f64 = x.sin();
        let sin_q = sin_q32(x_q32) as f64 / scale;
        max_err_sin = max_err_sin.max((sin_f64 - sin_q).abs());
        
        let cos_f64 = x.cos();
        let cos_q = cos_q32(x_q32) as f64 / scale;
        max_err_cos = max_err_cos.max((cos_f64 - cos_q).abs());
    }
    
    // Test sqrt
    for i in 0..=10000 {
        let x = (i as f64) * 0.1;
        let x_q32 = (x * scale).round() as i64;
        
        let sqrt_f64 = x.sqrt();
        let sqrt_q = sqrt_q32(x_q32) as f64 / scale;
        max_err_sqrt = max_err_sqrt.max((sqrt_f64 - sqrt_q).abs());
    }
    
    println!("Max err sin:  {:e}", max_err_sin);
    println!("Max err cos:  {:e}", max_err_cos);
    println!("Max err sqrt: {:e}", max_err_sqrt);
}
