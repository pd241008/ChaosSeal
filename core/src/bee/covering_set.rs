#[derive(Clone, Debug)]
pub struct CoveringSet {
    pub nodes: Vec<usize>,
}

impl CoveringSet {
    pub fn compute(n: usize, revoked: &[bool]) -> Self {
        let mut nodes = Vec::new();
        let mut i = 0;
        while i < n {
            if !revoked[i] {
                let mut j = i + 1;
                while j < n && !revoked[j] && (j & (j - 1)) != 0 {
                    j += 1;
                }
                nodes.push(i);
                i = j;
            } else {
                i += 1;
            }
        }
        Self { nodes }
    }

    pub fn size(&self) -> usize { self.nodes.len() }
}
