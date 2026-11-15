use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;

/// Create the deterministic RNG used for all dealing.
///
/// Algorithm: xoshiro256** seeded via `seed_from_u64(seed)` (see rand_xoshiro docs).
/// Documentation: docs/DEAL_ALGORITHM.md (to be written in M1).
pub fn make_rng(seed: u64) -> Xoshiro256StarStar {
    Xoshiro256StarStar::seed_from_u64(seed)
}

/// Fisher-Yates shuffle. Deterministic given the RNG.
pub fn shuffle<T>(items: &mut [T], rng: &mut impl Rng) {
    for i in (1..items.len()).rev() {
        let j = rng.gen_range(0..=i);
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shuffle_is_deterministic() {
        let mut a: Vec<u8> = (0..52).collect();
        let mut b: Vec<u8> = (0..52).collect();
        shuffle(&mut a, &mut make_rng(12345));
        shuffle(&mut b, &mut make_rng(12345));
        assert_eq!(a, b);
    }

    #[test]
    fn shuffle_is_a_permutation() {
        let mut a: Vec<u8> = (0..52).collect();
        shuffle(&mut a, &mut make_rng(1));
        let mut sorted = a.clone();
        sorted.sort();
        assert_eq!(sorted, (0..52).collect::<Vec<u8>>());
    }
}
