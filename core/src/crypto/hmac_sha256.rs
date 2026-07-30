use hmac::{Hmac, Mac};
use sha2::Sha256;

pub type HmacSha256 = Hmac<Sha256>;

pub fn compute(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key valid");
    mac.update(message);
    let result = mac.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result.into_bytes());
    out
}

pub fn verify(key: &[u8], message: &[u8], expected: &[u8; 32]) -> bool {
    let computed = compute(key, message);
    computed == *expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_kat_rfc4231_case_1() {
        let key = [0x0B; 20];
        let message = b"Hi There";
        let expected: [u8; 32] = [
            0xB0, 0x34, 0x4C, 0x61, 0xD8, 0xDB, 0x38, 0x53,
            0x5C, 0xA8, 0xAF, 0xCE, 0xAF, 0x0B, 0xF1, 0x2B,
            0x88, 0x1D, 0xC2, 0x00, 0xC9, 0x83, 0x3D, 0xA7,
            0x26, 0xE9, 0x37, 0x6C, 0x2E, 0x32, 0xCF, 0xF7,
        ];
        let computed = compute(&key, message);
        assert_eq!(computed, expected, "HMAC-SHA256 RFC 4231 case 1 mismatch");
        assert!(verify(&key, message, &expected));
    }

    #[test]
    fn test_hmac_kat_rfc4231_case_2() {
        let key = b"Jefe";
        let message = b"what do ya want for nothing?";
        let expected: [u8; 32] = [
            0x5B, 0xDC, 0xC1, 0x46, 0xBF, 0x60, 0x75, 0x4E,
            0x6A, 0x04, 0x24, 0x26, 0x08, 0x95, 0x75, 0xC7,
            0x5A, 0x00, 0x3F, 0x08, 0x9D, 0x27, 0x39, 0x83,
            0x9D, 0xEC, 0x58, 0xB9, 0x64, 0xEC, 0x38, 0x43,
        ];
        let computed = compute(key, message);
        assert_eq!(computed, expected, "HMAC-SHA256 RFC 4231 case 2 mismatch");
    }
}
