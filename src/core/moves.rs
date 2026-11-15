use crate::core::rules::{BuildOrder, EmptyPile, MoveGroup, Redeals, RuleSet};
use crate::core::state::{GameState, PileRef, PlacedCard};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Move {
    Draw,
    Recycle,
    Move { from: PileRef, from_idx: u8, to: PileRef },
    FlipTableau(u8),
    AutoFoundation(PileRef),
}

pub fn is_legal(s: &GameState, r: &RuleSet, m: &Move) -> bool {
    match m {
        Move::Draw => !s.stock.is_empty(),
        Move::Recycle => {
            if !s.stock.is_empty() || s.waste.is_empty() { return false; }
            match r.stock.redeals {
                Redeals::Infinite => true,
                Redeals::Count(n) => s.redeals_used < n,
            }
        }
        Move::Move { from, from_idx, to } => is_legal_move(s, r, *from, *from_idx, *to),
        Move::FlipTableau(t) => {
            let idx = *t as usize;
            idx < s.tableau.len()
                && matches!(s.tableau[idx].last(), Some(PlacedCard { face_up: false, .. }))
        }
        Move::AutoFoundation(from) => top_card_of(s, *from)
            .map(|c| can_place_on_foundation_anywhere(s, r, c))
            .unwrap_or(false),
    }
}

/// Top card of a pile, regardless of pile kind.
pub fn top_card_of(s: &GameState, r: PileRef) -> Option<crate::core::card::Card> {
    match r {
        PileRef::Tableau(i) => s.tableau[i as usize].last().map(|p| p.card),
        PileRef::Foundation(i) => s.foundation[i as usize].last().copied(),
        PileRef::Stock => s.stock.last().copied(),
        PileRef::Waste => s.waste.last().copied(),
        PileRef::FreeCell(i) => s.free_cells[i as usize],
    }
}

fn is_legal_move(s: &GameState, r: &RuleSet, from: PileRef, from_idx: u8, to: PileRef) -> bool {
    if from == to { return false; }

    let fi = from_idx as usize;

    // Fetch the cards we'd be moving (in order, bottom → top).
    let cards: Vec<crate::core::card::Card> = match from {
        PileRef::Tableau(t) => {
            let pile = &s.tableau[t as usize];
            if fi >= pile.len() { return false; }
            match r.tableau.move_group {
                MoveGroup::OnlySingle if fi + 1 != pile.len() => return false,
                MoveGroup::OnlyFaceUp if !pile[fi].face_up => return false,
                MoveGroup::Any if !pile[fi].face_up => return false,
                _ => {}
            }
            if r.tableau.move_group == MoveGroup::Any
                && !is_valid_run(&pile[fi..], r.tableau.build, r.tableau.alternate_color)
            {
                return false;
            }
            pile[fi..].iter().map(|p| p.card).collect()
        }
        PileRef::Waste => {
            if fi + 1 != s.waste.len() || s.waste.is_empty() { return false; }
            vec![s.waste[s.waste.len() - 1]]
        }
        PileRef::FreeCell(f) => {
            if fi != 0 { return false; }
            match s.free_cells[f as usize] {
                Some(c) => vec![c],
                None => return false,
            }
        }
        PileRef::Foundation(_) | PileRef::Stock => return false,
    };

    if cards.is_empty() { return false; }

    // Destination checks.
    match to {
        PileRef::Tableau(ti) => {
            let ti = ti as usize;
            if ti >= s.tableau.len() { return false; }
            let dest = &s.tableau[ti];
            let first = cards[0];
            match dest.last() {
                None => {
                    if !empty_pile_accepts(r.tableau.empty_pile_accepts, first.rank.value()) {
                        return false;
                    }
                }
                Some(top) => {
                    if !top.face_up { return false; }
                    if !build_accepts(r.tableau.build, r.tableau.alternate_color, top.card, first) {
                        return false;
                    }
                }
            }
            true
        }
        PileRef::Foundation(fi) => {
            let fi = fi as usize;
            if fi >= s.foundation.len() { return false; }
            if cards.len() != 1 { return false; }
            let c = cards[0];
            let pile = &s.foundation[fi];
            match pile.last() {
                None => c.rank.value() == r.foundation.start_rank.as_value(),
                Some(top) => {
                    if r.foundation.same_suit && top.suit != c.suit { return false; }
                    match r.foundation.build {
                        BuildOrder::Ascending => c.rank.value() == top.rank.value() + 1,
                        BuildOrder::Descending => c.rank.value() + 1 == top.rank.value(),
                    }
                }
            }
        }
        PileRef::FreeCell(fi) => {
            if cards.len() != 1 { return false; }
            fi < s.free_cells.len() as u8 && s.free_cells[fi as usize].is_none()
        }
        PileRef::Stock | PileRef::Waste => false,
    }
}

/// Is `pile` a valid descending (or ascending) alternating-color run?
/// All cards in the run must be face-up.
fn is_valid_run(pile: &[PlacedCard], build: BuildOrder, alternate_color: bool) -> bool {
    if pile.len() <= 1 {
        return pile.first().map(|p| p.face_up).unwrap_or(true);
    }
    for w in pile.windows(2) {
        let a_placed = &w[0];
        let b_placed = &w[1];
        if !a_placed.face_up || !b_placed.face_up { return false; }
        let a = a_placed.card;
        let b = b_placed.card;
        let rank_ok = match build {
            BuildOrder::Descending => b.rank.value() + 1 == a.rank.value(),
            BuildOrder::Ascending => a.rank.value() + 1 == b.rank.value(),
        };
        let color_ok = if alternate_color { a.suit.is_red() != b.suit.is_red() } else { true };
        if !rank_ok || !color_ok { return false; }
    }
    true
}

fn build_accepts(
    build: BuildOrder,
    alternate_color: bool,
    top: crate::core::card::Card,
    next: crate::core::card::Card,
) -> bool {
    let rank_ok = match build {
        BuildOrder::Descending => next.rank.value() + 1 == top.rank.value(),
        BuildOrder::Ascending => top.rank.value() + 1 == next.rank.value(),
    };
    let color_ok = if alternate_color { top.suit.is_red() != next.suit.is_red() } else { true };
    rank_ok && color_ok
}

fn empty_pile_accepts(rule: EmptyPile, rank_value: u8) -> bool {
    match rule {
        EmptyPile::Any => true,
        EmptyPile::King => rank_value == 13,
        EmptyPile::Ace => rank_value == 1,
        EmptyPile::None => false,
    }
}

fn can_place_on_foundation_anywhere(
    s: &GameState,
    r: &RuleSet,
    c: crate::core::card::Card,
) -> bool {
    for fi in 0..s.foundation.len() {
        let pile = &s.foundation[fi];
        let ok = match pile.last() {
            None => c.rank.value() == r.foundation.start_rank.as_value(),
            Some(top) => {
                (r.foundation.same_suit && top.suit == c.suit || !r.foundation.same_suit)
                    && match r.foundation.build {
                        BuildOrder::Ascending => c.rank.value() == top.rank.value() + 1,
                        BuildOrder::Descending => c.rank.value() + 1 == top.rank.value(),
                    }
            }
        };
        if ok { return true; }
    }
    false
}
