use super::*;

pub fn serialize_ciphertext(_engine: &BEEEngine, covering: &[usize]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(covering.len() as u32).to_be_bytes());
    for &node in covering {
        out.extend_from_slice(&(node as u32).to_be_bytes());
    }
    for _ in 0..covering.len() {
        out.extend_from_slice(&[0u8; 32]);
    }
    out
}

pub fn deserialize_ciphertext(data: &[u8]) -> (usize, Vec<usize>, Vec<u8>) {
    let count = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let mut nodes = Vec::with_capacity(count);
    for i in 0..count {
        let offset = 4 + i * 4;
        let node = u32::from_be_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]) as usize;
        nodes.push(node);
    }
    let keys_start = 4 + count * 4;
    let keys = data[keys_start..keys_start + count * 32].to_vec();
    (count, nodes, keys)
}
