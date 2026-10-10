//! Deterministic state hashing (FNV-1a 64). Fixed forever: changing it is a
//! re-golden event.

/// Incremental FNV-1a 64-bit hasher over explicit little-endian bytes.
#[derive(Debug, Clone)]
pub struct StateHasher(u64);

impl Default for StateHasher {
    fn default() -> Self {
        StateHasher(0xcbf2_9ce4_8422_2325)
    }
}

impl StateHasher {
    /// Feed bytes.
    pub fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
    /// Feed an `f64` by its exact bit pattern.
    pub fn write_f64(&mut self, x: f64) {
        self.write(&x.to_bits().to_le_bytes());
    }
    /// The hash value.
    #[must_use]
    pub fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fnv_known_answer() {
        let mut h = StateHasher::default();
        h.write(b"a");
        assert_eq!(h.finish(), 0xaf63_dc4c_8601_ec8c);
    }
}
