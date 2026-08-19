use chaosseal_core::crypto::AesGcmCipher;
fn main() {
    let payload = vec![0u8; 1024];
    let cipher = AesGcmCipher::new([0u8; 32]);
    let nonce = AesGcmCipher::random_nonce();
    let ct = cipher.encrypt(&payload, &nonce);
    println!("Ciphertext len: {}", ct.len());
}
