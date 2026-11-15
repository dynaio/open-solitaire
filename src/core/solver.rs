//! Greedy solvability check.
//!
//! Klondike solvability is NP-hard; we do not try to prove unsolvable.
//! Instead we run a hint-driven bot. If the bot wins the seed, the seed
//! IS solvable (a constructive proof). If the bot loses or times out, we
//! return `false` and the caller retries with a different seed.

use crate::core::engine::Engine;
use crate::core::rules::RuleSet;

/// Try to win the game started from `seed` using greedy hint play.
/// Returns `true` only if we reach a win state.
pub fn try_solve(rule: &RuleSet, seed: u64, max_moves: usize) -> bool {
    let mut engine = Engine::new(rule.clone(), seed);
    for _ in 0..max_moves {
        if engine.state.is_won() { return true; }
        match engine.hint() {
            Some(m) => {
                if !engine.apply(m) {
                    // Hint returned an inapplicable move — treat as dead end.
                    return false;
                }
            }
            None => return engine.state.is_won(),
        }
    }
    engine.state.is_won()
}

/// Find the first solvable seed at or after `start`.
/// Tries up to `max_tries` consecutive seeds. Falls back to `start` if
/// every attempt fails (extremely unlikely for Klondike).
pub fn find_solvable_seed(rule: &RuleSet, start: u64, max_tries: u64) -> u64 {
    for i in 0..max_tries {
        let seed = start.wrapping_add(i);
        if try_solve(rule, seed, 500) {
            return seed;
        }
    }
    start
}

#[cfg(test)]
mod tests {
    use super::*;

    fn klondike() -> RuleSet {
        RuleSet::from_json(&std::fs::read_to_string("assets/rules/klondike.json").unwrap()).unwrap()
    }

    #[test]
    fn solvable_seed_is_actually_solvable() {
        let rule = klondike();
        let seed = find_solvable_seed(&rule, 1, 100);
        // The returned seed must pass try_solve directly.
        assert!(try_solve(&rule, seed, 500),
                "find_solvable_seed returned {} which fails try_solve", seed);
    }
}
