use crate::core::moves::{is_legal, Move};
use crate::core::rules::RuleSet;
use crate::core::state::{GameState, PileRef, PlacedCard};

pub const MAX_UNDO_DEPTH: usize = 500;

pub struct Engine {
    pub state: GameState,
    pub rules: RuleSet,
    history: Vec<GameState>,
}

impl Engine {
    pub fn new(rules: RuleSet, seed: u64) -> Self {
        let state = GameState::new(&rules, seed);
        Engine { state, rules, history: Vec::new() }
    }

    pub fn can(&self, m: &Move) -> bool { is_legal(&self.state, &self.rules, m) }

    /// Apply a move. Returns `true` if it was legal and applied.
    pub fn apply(&mut self, m: Move) -> bool {
        if !self.can(&m) { return false; }

        self.history.push(self.state.clone());
        if self.history.len() > MAX_UNDO_DEPTH {
            self.history.remove(0);
        }

        match m {
            Move::Draw => self.do_draw(),
            Move::Recycle => self.do_recycle(),
            Move::FlipTableau(t) => self.do_flip(t),
            Move::Move { from, from_idx, to } => self.do_move(from, from_idx, to),
            Move::AutoFoundation(from) => self.do_auto_foundation(from),
        }
        true
    }

    pub fn undo(&mut self) -> bool {
        match self.history.pop() {
            Some(prev) => { self.state = prev; true }
            None => false,
        }
    }

    // ---- Move implementations ----

    fn do_draw(&mut self) {
        let n = (self.rules.stock.draw as usize).min(self.state.stock.len());
        for _ in 0..n {
            if let Some(c) = self.state.stock.pop() {
                self.state.waste.push(c);
            }
        }
    }

    fn do_recycle(&mut self) {
        // Reverse waste back into stock (bottom card of waste becomes top of stock).
        while let Some(c) = self.state.waste.pop() {
            self.state.stock.push(c);
        }
        self.state.redeals_used += 1;
    }

    fn do_flip(&mut self, t: u8) {
        if let Some(last) = self.state.tableau[t as usize].last_mut() {
            last.face_up = true;
        }
    }

    fn do_move(&mut self, from: PileRef, from_idx: u8, to: PileRef) {
        let cards = self.take_from(from, from_idx as usize);
        self.put_on(to, cards);

        // Auto-flip new top of source tableau if it's face-down.
        if let PileRef::Tableau(t) = from {
            if let Some(top) = self.state.tableau[t as usize].last_mut() {
                if !top.face_up { top.face_up = true; }
            }
        }

        // Score: +10 for card to foundation.
        if let PileRef::Foundation(_) = to {
            self.state.score += 10;
        }
    }

    fn do_auto_foundation(&mut self, from: PileRef) {
        // Find first foundation pile that accepts the top card.
        let top = match crate::core::moves::top_card_of(&self.state, from) {
            Some(c) => c,
            None => return,
        };
        for fi in 0..self.state.foundation.len() {
            let target = PileRef::Foundation(fi as u8);
            let m = Move::Move { from, from_idx: (self.state.pile_len(from) - 1) as u8, to: target };
            if is_legal(&self.state, &self.rules, &m) {
                let _ = self.apply_move_inner(from, top, target);
                return;
            }
        }
    }

    fn apply_move_inner(&mut self, from: PileRef, _top: crate::core::card::Card, to: PileRef) -> bool {
        let idx = (self.state.pile_len(from) - 1) as u8;
        let cards = self.take_from(from, idx as usize);
        self.put_on(to, cards);
        if let PileRef::Tableau(t) = from {
            if let Some(top) = self.state.tableau[t as usize].last_mut() {
                if !top.face_up { top.face_up = true; }
            }
        }
        if let PileRef::Foundation(_) = to { self.state.score += 10; }
        true
    }

    fn take_from(&mut self, from: PileRef, idx: usize) -> Vec<crate::core::card::Card> {
        match from {
            PileRef::Tableau(t) => self.state.tableau[t as usize]
                .drain(idx..)
                .map(|p| p.card)
                .collect(),
            PileRef::Waste => {
                self.state.waste.pop().into_iter().collect()
            }
            PileRef::FreeCell(f) => self.state.free_cells[f as usize].take().into_iter().collect(),
            _ => Vec::new(),
        }
    }

    fn put_on(&mut self, to: PileRef, cards: Vec<crate::core::card::Card>) {
        match to {
            PileRef::Tableau(t) => {
                for c in cards {
                    self.state.tableau[t as usize].push(PlacedCard { card: c, face_up: true });
                }
            }
            PileRef::Foundation(f) => {
                for c in cards {
                    self.state.foundation[f as usize].push(c);
                }
            }
            PileRef::FreeCell(f) => {
                if let Some(c) = cards.into_iter().next() {
                    self.state.free_cells[f as usize] = Some(c);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::moves::Move;
    use crate::core::rules::RuleSet;

    fn klondike() -> RuleSet {
        RuleSet::from_json(&std::fs::read_to_string("assets/rules/klondike.json").unwrap()).unwrap()
    }

    #[test]
    fn draw_moves_card_to_waste() {
        let mut e = Engine::new(klondike(), 1);
        let stock_before = e.state.stock.len();
        assert!(e.apply(Move::Draw));
        assert_eq!(e.state.stock.len(), stock_before - 1);
        assert_eq!(e.state.waste.len(), 1);
    }

    #[test]
    fn undo_restores_state() {
        let mut e = Engine::new(klondike(), 42);
        let snapshot = format!("{:?}", e.state);
        e.apply(Move::Draw);
        assert_ne!(format!("{:?}", e.state), snapshot);
        assert!(e.undo());
        assert_eq!(format!("{:?}", e.state), snapshot);
    }

    #[test]
    fn double_undo_is_safe() {
        let mut e = Engine::new(klondike(), 42);
        assert!(!e.undo());
        e.apply(Move::Draw);
        e.undo();
        assert!(!e.undo());
    }

    #[test]
    fn illegal_move_is_rejected() {
        let mut e = Engine::new(klondike(), 42);
        // Stock is not empty so Recycle is illegal.
        assert!(!e.apply(Move::Recycle));
    }

    #[test]
    fn top_card_face_up_after_deal() {
        let e = Engine::new(klondike(), 1);
        for pile in &e.state.tableau {
            assert!(pile.last().unwrap().face_up);
        }
    }
}
