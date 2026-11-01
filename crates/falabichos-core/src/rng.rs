// SPDX-License-Identifier: Apache-2.0
//! Tiny deterministic PRNG (SplitMix64). Deterministic so tests, headless sims and saves reproduce.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }
    pub fn state(&self) -> u64 {
        self.state
    }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n` (returns 0 when `n == 0`).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }
    /// True with `percent`% probability.
    pub fn chance(&mut self, percent: u8) -> bool {
        self.below(100) < percent as usize
    }
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
    pub fn pick<'a, T>(&mut self, v: &'a [T]) -> Option<&'a T> {
        if v.is_empty() {
            None
        } else {
            v.get(self.below(v.len()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..50 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn shuffle_is_permutation() {
        let mut r = Rng::new(1);
        let mut v: Vec<u32> = (0..20).collect();
        r.shuffle(&mut v);
        let mut s = v.clone();
        s.sort();
        assert_eq!(s, (0..20).collect::<Vec<_>>());
    }

    #[test]
    fn below_in_range() {
        let mut r = Rng::new(3);
        for _ in 0..1000 {
            assert!(r.below(7) < 7);
        }
        assert_eq!(r.below(0), 0);
    }
}
