//! Deterministic DFS solver with heuristic move ordering.
//!
//! Klondike solvability is NP-hard. We use a bounded DFS:
//!   - Move ordering: foundation > reveal > waste > others > draw
//!   - Visited set of state signatures prevents cycles
//!   - Deterministic bound (`max_states`) so tests are reproducible
//!
//! If DFS finds a solution, we return the move path. The `Engine`
//! stores that path so `hint()` becomes a guaranteed winner: each
//! successive hint is the next move in a real solution.

use std::collections::HashSet;

use crate::core::engine::Engine;
use crate::core::moves::Move;
use crate::core::rules::RuleSet;
use crate::core::state::{GameState, PileRef};

/// Compact FNV-1a hash of the game state. Used for the visited set.
fn state_signature(s: &GameState) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mix = |h: u64, v: u64| (h ^ v).wrapping_mul(0x100000001b3);

    for pile in &s.tableau {
        for pc in pile {
            h = mix(h, pc.card.id() as u64 | if pc.face_up { 0x40 } else { 0 });
        }
        h = mix(h, 0xFF);
    }
    for c in &s.stock {
        h = mix(h, c.id() as u64);
    }
    h = mix(h, 0xFE);
    for c in &s.waste {
        h = mix(h, c.id() as u64);
    }
    h = mix(h, 0xFD);
    for p in &s.foundation {
        for c in p {
            h = mix(h, c.id() as u64);
        }
        h = mix(h, 0xFC);
    }
    h
}

fn enumerate_moves(e: &Engine) -> Vec<Move> {
    let s = &e.state;
    let mut moves = Vec::new();

    if !s.stock.is_empty() {
        moves.push(Move::Draw);
    } else if !s.waste.is_empty() {
        let m = Move::Recycle;
        if e.can(&m) {
            moves.push(m);
        }
    }

    for t in 0..s.tableau.len() {
        if let Some(top) = s.tableau[t].last() {
            if top.face_up {
                for f in 0..s.foundation.len() {
                    let m = Move::Move {
                        from: PileRef::Tableau(t as u8),
                        from_idx: (s.tableau[t].len() - 1) as u8,
                        to: PileRef::Foundation(f as u8),
                    };
                    if e.can(&m) {
                        moves.push(m);
                    }
                }
            }
        }
    }

    if !s.waste.is_empty() {
        for f in 0..s.foundation.len() {
            let m = Move::Move {
                from: PileRef::Waste,
                from_idx: (s.waste.len() - 1) as u8,
                to: PileRef::Foundation(f as u8),
            };
            if e.can(&m) {
                moves.push(m);
            }
        }
    }

    for t1 in 0..s.tableau.len() {
        let pile = &s.tableau[t1];
        for i in 0..pile.len() {
            if !pile[i].face_up {
                continue;
            }
            for t2 in 0..s.tableau.len() {
                if t1 == t2 {
                    continue;
                }
                let m = Move::Move {
                    from: PileRef::Tableau(t1 as u8),
                    from_idx: i as u8,
                    to: PileRef::Tableau(t2 as u8),
                };
                if e.can(&m) {
                    moves.push(m);
                }
            }
        }
    }

    if !s.waste.is_empty() {
        for t in 0..s.tableau.len() {
            let m = Move::Move {
                from: PileRef::Waste,
                from_idx: (s.waste.len() - 1) as u8,
                to: PileRef::Tableau(t as u8),
            };
            if e.can(&m) {
                moves.push(m);
            }
        }
    }

    moves
}

fn move_score(m: &Move, s: &GameState) -> i32 {
    match m {
        Move::Move {
            to: PileRef::Foundation(_),
            ..
        } => 100,
        Move::Move {
            from: PileRef::Tableau(t),
            from_idx,
            ..
        } => {
            let pile = &s.tableau[*t as usize];
            let reveals = (*from_idx as usize) > 0
                && pile
                    .get((*from_idx as usize) - 1)
                    .map(|p| !p.face_up)
                    .unwrap_or(false);
            if reveals {
                90
            } else {
                40
            }
        }
        Move::Move {
            from: PileRef::Waste,
            ..
        } => 60,
        Move::Draw => 20,
        Move::Recycle => 10,
        _ => 0,
    }
}

fn dfs(
    engine: &mut Engine,
    visited: &mut HashSet<u64>,
    path: &mut Vec<Move>,
    max_depth: usize,
    max_states: usize,
    state_count: &mut usize,
) -> Option<Vec<Move>> {
    if engine.state.is_won() {
        return Some(path.clone());
    }
    if path.len() >= max_depth || *state_count >= max_states {
        return None;
    }

    let sig = state_signature(&engine.state);
    if !visited.insert(sig) {
        return None;
    }
    *state_count += 1;

    let mut moves = enumerate_moves(engine);
    moves.sort_by_key(|m| -move_score(m, &engine.state));

    for m in moves {
        let saved = engine.state.clone();
        if engine.apply_no_history(&m) {
            path.push(m.clone());
            if let Some(sol) = dfs(engine, visited, path, max_depth, max_states, state_count) {
                return Some(sol);
            }
            path.pop();
        }
        engine.state = saved;
    }
    None
}

/// Solve from a fresh deal created by `seed`.
pub fn solve(rule: &RuleSet, seed: u64, max_states: usize) -> Option<Vec<Move>> {
    let state = GameState::new(rule, seed);
    solve_from_state(&state, rule, max_states)
}

/// Solve starting from an arbitrary state (used to resume after a user move).
pub fn solve_from_state(
    state: &GameState,
    rule: &RuleSet,
    max_states: usize,
) -> Option<Vec<Move>> {
    let mut engine = Engine::new(rule.clone(), state.deal_seed);
    engine.state = state.clone();
    let mut visited = HashSet::new();
    let mut path = Vec::new();
    let mut count = 0;
    dfs(&mut engine, &mut visited, &mut path, 500, max_states, &mut count)
}

/// Find the first solvable seed at or after `start`.
pub fn find_solvable_seed(rule: &RuleSet, start: u64, max_tries: u64) -> u64 {
    for i in 0..max_tries {
        let seed = start.wrapping_add(i);
        if solve(rule, seed, 50_000).is_some() {
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
        let seed = find_solvable_seed(&rule, 1, 10);
        assert!(
            solve(&rule, seed, 50_000).is_some(),
            "find_solvable_seed returned {} but solve failed",
            seed
        );
    }
}
