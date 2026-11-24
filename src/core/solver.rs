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
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::engine::Engine;
use crate::core::moves::Move;
use crate::core::rules::RuleSet;
use crate::core::state::{GameState, PileRef};

/// Stable FNV-1a signature of every field that can affect legal moves or victory.
pub fn state_signature(s: &GameState) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut mix = |v: u64| {
        h = (h ^ v).wrapping_mul(0x100000001b3);
    };
    for b in s.rule_id.as_bytes() {
        mix(*b as u64);
    }
    mix(0xF1);
    for pile in &s.tableau {
        mix(pile.len() as u64);
        for pc in pile {
            mix(pc.card.id() as u64);
            mix(pc.face_up as u64);
        }
        mix(0xFF);
    }
    mix(s.foundation.len() as u64);
    for pile in &s.foundation {
        mix(pile.len() as u64);
        for c in pile {
            mix(c.id() as u64);
        }
        mix(0xFE);
    }
    mix(s.stock.len() as u64);
    for c in &s.stock {
        mix(c.id() as u64);
    }
    mix(0xFD);
    mix(s.waste.len() as u64);
    for c in &s.waste {
        mix(c.id() as u64);
    }
    mix(0xFC);
    mix(s.free_cells.len() as u64);
    for cell in &s.free_cells {
        mix(cell.map(|c| c.id() as u64 + 1).unwrap_or(0));
    }
    mix(s.redeals_used as u64);
    h
}

/// Hash the complete rule configuration, including difficulty-adjusted settings.
pub fn rules_signature(rule: &RuleSet) -> u64 {
    let bytes = serde_json::to_vec(rule).unwrap_or_default();
    let mut h: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        h = (h ^ byte as u64).wrapping_mul(0x100000001b3);
    }
    h
}

fn supports_solver(rule: &RuleSet) -> bool {
    matches!(rule.id.as_str(), "klondike" | "klondike-draw3")
}

fn cache_path(state: &GameState, rule: &RuleSet) -> Option<std::path::PathBuf> {
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share"))
        })?;
    let key = state_signature(state) ^ rules_signature(rule).rotate_left(17);
    Some(
        root.join("openpatience")
            .join("solutions")
            .join(format!("{key:016x}.json")),
    )
}

fn solution_wins(state: &GameState, rule: &RuleSet, solution: &[Move]) -> bool {
    let mut engine = Engine::new(rule.clone(), state.deal_seed);
    engine.state = state.clone();
    for movement in solution {
        if !engine.apply_no_history(movement) {
            return false;
        }
    }
    engine.state.is_won()
}

fn load_cached_solution(state: &GameState, rule: &RuleSet) -> Option<Vec<Move>> {
    let path = cache_path(state, rule)?;
    let bytes = std::fs::read(path).ok()?;
    let solution: Vec<Move> = serde_json::from_slice(&bytes).ok()?;
    solution_wins(state, rule, &solution).then_some(solution)
}

fn store_cached_solution(state: &GameState, rule: &RuleSet, solution: &[Move]) {
    let Some(path) = cache_path(state, rule) else {
        return;
    };
    let Some(parent) = path.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(bytes) = serde_json::to_vec(solution) else {
        return;
    };
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    if std::fs::write(&temp, bytes).is_ok() {
        let _ = std::fs::rename(temp, path);
    }
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
    cancelled: Option<&AtomicBool>,
) -> Option<Vec<Move>> {
    if cancelled
        .map(|flag| flag.load(Ordering::Relaxed))
        .unwrap_or(false)
    {
        return None;
    }
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
            if let Some(sol) = dfs(
                engine,
                visited,
                path,
                max_depth,
                max_states,
                state_count,
                cancelled,
            ) {
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
pub fn solve_from_state(state: &GameState, rule: &RuleSet, max_states: usize) -> Option<Vec<Move>> {
    solve_internal(state, rule, max_states, None)
}

/// Cancellable variant for interactive callers. Cancellation is checked throughout DFS.
pub fn solve_from_state_cancellable(
    state: &GameState,
    rule: &RuleSet,
    max_states: usize,
    cancelled: &AtomicBool,
) -> Option<Vec<Move>> {
    solve_internal(state, rule, max_states, Some(cancelled))
}

fn solve_internal(
    state: &GameState,
    rule: &RuleSet,
    max_states: usize,
    cancelled: Option<&AtomicBool>,
) -> Option<Vec<Move>> {
    if !supports_solver(rule)
        || state.rule_id != rule.id
        || cancelled
            .map(|flag| flag.load(Ordering::Relaxed))
            .unwrap_or(false)
    {
        return None;
    }
    if let Some(solution) = load_cached_solution(state, rule) {
        return Some(solution);
    }

    let mut engine = Engine::new(rule.clone(), state.deal_seed);
    engine.state = state.clone();
    let mut visited = HashSet::new();
    let mut path = Vec::new();
    let mut count = 0;
    let solution = dfs(
        &mut engine,
        &mut visited,
        &mut path,
        500,
        max_states,
        &mut count,
        cancelled,
    );
    if let Some(ref moves) = solution {
        store_cached_solution(state, rule, moves);
    }
    solution
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
    fn state_signature_includes_free_cells_and_redeals() {
        let rule = klondike();
        let original = GameState::new(&rule, 123);
        let mut changed = original.clone();
        changed
            .free_cells
            .push(Some(crate::core::card::Card::full_deck()[0]));
        assert_ne!(state_signature(&original), state_signature(&changed));

        let mut changed = original.clone();
        changed.redeals_used += 1;
        assert_ne!(state_signature(&original), state_signature(&changed));
    }

    #[test]
    fn solver_rejects_unsupported_variants() {
        let rule =
            RuleSet::from_json(&std::fs::read_to_string("assets/rules/freecell.json").unwrap())
                .unwrap();
        let state = GameState::new(&rule, 123);
        assert!(solve_from_state(&state, &rule, 1_000).is_none());
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
