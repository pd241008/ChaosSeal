use crate::crypto::hmac_sha256;
use rand::RngCore;

#[derive(Clone, Debug)]
pub struct BEEEngine {
    pub n: usize,
    pub r: usize,
    pub key_size_bits: usize,
    pub ciphertext_overhead_bytes: usize,
}

impl Default for BEEEngine {
    fn default() -> Self {
        Self {
            n: 1024,
            r: 8,
            key_size_bits: 256,
            ciphertext_overhead_bytes: 32,
        }
    }
}

impl BEEEngine {
    pub fn new(n: usize, r: usize) -> Self {
        Self {
            n,
            r,
            key_size_bits: 256,
            ciphertext_overhead_bytes: 32,
        }
    }

    /// Calculate the covering set size for a join event.
    /// A join at N+1 with r revoked requires rebuilding the tree to include the new leaf.
    /// The worst-case broadcast size equals the covering set size for r revoked at N+1.
    pub fn join_covering_set_size(&self, new_n: usize, revoked: &[bool]) -> usize {
        if revoked.iter().all(|&x| !x) {
            return 1;
        }
        let mut count = 0;
        let mut i = 0;
        while i < new_n {
            if !revoked[i] {
                let mut j = i + 1;
                while j < new_n && !revoked[j] && (j & (j - 1)) != 0 {
                    j += 1;
                }
                count += 1;
                i = j;
            } else {
                i += 1;
            }
        }
        count
    }

    /// Minimum covering set size for join (analytical formula).
    /// Cost of join is same as revocation: ceil(log(N+1)/log(r)) * r blocks
    pub fn join_covering_set_min(&self, new_n: usize) -> usize {
        if self.r == 0 { return 1; }
        if self.r == 1 { return 1; }
        let log_n = (new_n as f64).log2().ceil();
        let log_r = (self.r as f64).log2().ceil();
        let m = (log_n / log_r).ceil() as usize;
        m * self.r
    }

    /// Join broadcast size in bytes (analytical minimum)
    pub fn join_broadcast_bytes(&self, new_n: usize) -> usize {
        let covering = self.join_covering_set_min(new_n);
        covering * (self.key_size_bits / 8 + self.ciphertext_overhead_bytes)
    }

    /// Amortized per-join cost under lazy rebuild schedule
    pub fn amortized_join_bytes(&self, new_n: usize, join_rate_per_epoch: f64, rebuild_interval_epochs: f64) -> f64 {
        let total_bytes = self.join_broadcast_bytes(new_n) as f64;
        let joins_per_rebuild = join_rate_per_epoch * rebuild_interval_epochs;
        if joins_per_rebuild > 0.0 {
            total_bytes / joins_per_rebuild
        } else {
            total_bytes
        }
    }

    /// Per-epoch overhead bytes under lazy rebuild schedule
    pub fn epoch_overhead_bytes(&self, new_n: usize, _join_rate_per_epoch: f64, rebuild_interval_epochs: f64) -> f64 {
        let total_bytes = self.join_broadcast_bytes(new_n) as f64;
        total_bytes / rebuild_interval_epochs
    }

    pub fn build_key_tree(&self) -> Vec<Vec<Vec<u8>>> {
        let levels = (self.n as f64).log2().ceil() as usize;
        let mut tree: Vec<Vec<Vec<u8>>> = vec![vec![vec![0u8; 32]; self.n * 2]; levels + 1];
        let mut rng = rand::thread_rng();
        for i in 0..self.n {
            tree[0][i] = (0..32).map(|_| rng.next_u32() as u8).collect();
        }
        for level in 1..=levels {
            let nodes_at_level = self.n >> (level - 1);
            for i in 0..nodes_at_level {
                let left = &tree[level - 1][i * 2];
                let right = &tree[level - 1][i * 2 + 1];
                let mut node_key = vec![0u8; 32];
                for (j, (a, b)) in left.iter().zip(right.iter()).enumerate() {
                    node_key[j] = a ^ b;
                }
                tree[level][i] = node_key;
            }
        }
        tree
    }

    pub fn covering_set_size(&self, revoked: &[bool]) -> usize {
        if revoked.iter().all(|&x| !x) {
            return 1;
        }
        let mut count = 0;
        let mut i = 0;
        while i < self.n {
            if !revoked[i] {
                let mut j = i + 1;
                while j < self.n && !revoked[j] && (j & (j - 1)) != 0 {
                    j += 1;
                }
                count += 1;
                i = j;
            } else {
                i += 1;
            }
        }
        count
    }

    pub fn ciphertext_size(&self) -> usize {
        let covering = self.covering_set_min();
        covering * (self.key_size_bits / 8 + self.ciphertext_overhead_bytes)
    }

    pub fn ciphertext_size_min(&self) -> usize {
        if self.r == 0 { return self.ciphertext_overhead_bytes + 32; }
        let covering = self.covering_set_min();
        covering * (self.key_size_bits / 8 + self.ciphertext_overhead_bytes)
    }

    fn covering_set_min(&self) -> usize {
        if self.r == 0 { return 1; }
        if self.r == 1 { return 1; }
        let log_n = (self.n as f64).log2().ceil();
        let log_r = (self.r as f64).log2().ceil();
        let m = (log_n / log_r).ceil() as usize;
        m * self.r
    }

    pub fn simulation_key(&self) -> Vec<u8> {
        let seed = b"chaosseal-bee-simulation-seed";
        let key = hmac_sha256::compute(seed, &[self.n as u8, self.r as u8]);
        key.to_vec()
    }
}
