use crate::core::card::Card;
use crate::core::rng::{make_rng, shuffle};
use crate::core::rules::RuleSet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PileRef {
    Tableau(u8),
    Foundation(u8),
    Stock,
    Waste,
    FreeCell(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacedCard {
    pub card: Card,
    pub face_up: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub rule_id: String,
    pub deal_seed: u64,
    pub tableau: Vec<Vec<PlacedCard>>,
    pub foundation: Vec<Vec<Card>>,
    pub stock: Vec<Card>,
    pub waste: Vec<Card>,
    pub free_cells: Vec<Option<Card>>,
    pub redeals_used: u32,
    pub score: i32,
}

impl GameState {
    /// Deal a new game from `rule` and `seed`. Deterministic.
    pub fn new(rule: &RuleSet, seed: u64) -> Self {
        rule.validate().expect("invalid rule");

        let mut rng = make_rng(seed);
        let mut deck: Vec<Card> = Vec::with_capacity(rule.total_cards());
        for _ in 0..rule.decks {
            deck.extend(Card::full_deck());
        }
        shuffle(&mut deck, &mut rng);

        let n_tableau = rule.tableau.piles as usize;
        let pattern = &rule.tableau.initial_deal_pattern;
        let max_row = *pattern.iter().max().unwrap() as usize;

        let mut tableau: Vec<Vec<PlacedCard>> = (0..n_tableau).map(|_| Vec::new()).collect();

        // Classic row-by-row deal: row r deals to every column c where pattern[c] > r.
        let mut deck_idx = 0usize;
        for row in 0..max_row {
            for col in 0..n_tableau {
                if pattern[col] as usize > row {
                    let card = deck[deck_idx];
                    deck_idx += 1;
                    tableau[col].push(PlacedCard {
                        card,
                        face_up: false,
                    });
                }
            }
        }

        // Flip the last card of each tableau pile (if only_last_face_up).
        for pile in &mut tableau {
            if rule.tableau.only_last_face_up {
                if let Some(last) = pile.last_mut() {
                    last.face_up = true;
                }
            } else {
                for c in pile.iter_mut() {
                    c.face_up = true;
                }
            }
        }

        let foundation: Vec<Vec<Card>> = (0..rule.foundation.piles).map(|_| Vec::new()).collect();
        let free_cells: Vec<Option<Card>> = (0..rule.free_cells).map(|_| None).collect();
        let stock: Vec<Card> = deck[deck_idx..].to_vec();

        GameState {
            rule_id: rule.id.clone(),
            deal_seed: seed,
            tableau,
            foundation,
            stock,
            waste: Vec::new(),
            free_cells,
            redeals_used: 0,
            score: 0,
        }
    }

    /// True when every card is in the foundation.
    pub fn is_won(&self) -> bool {
        let f_count: usize = self.foundation.iter().map(|p| p.len()).sum();
        f_count == self.total_cards()
    }

    pub fn total_cards(&self) -> usize {
        self.tableau.iter().map(|p| p.len()).sum::<usize>()
            + self.foundation.iter().map(|p| p.len()).sum::<usize>()
            + self.stock.len()
            + self.waste.len()
            + self.free_cells.iter().filter(|c| c.is_some()).count()
    }

    pub fn pile_len(&self, r: PileRef) -> usize {
        match r {
            PileRef::Tableau(i) => self.tableau[i as usize].len(),
            PileRef::Foundation(i) => self.foundation[i as usize].len(),
            PileRef::Stock => self.stock.len(),
            PileRef::Waste => self.waste.len(),
            PileRef::FreeCell(i) => {
                if self.free_cells[i as usize].is_some() {
                    1
                } else {
                    0
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::rules::RuleSet;

    fn klondike() -> RuleSet {
        RuleSet::from_json(&std::fs::read_to_string("assets/rules/klondike.json").unwrap()).unwrap()
    }

    #[test]
    fn klondike_deal_layout() {
        let s = GameState::new(&klondike(), 12345);
        for i in 0..7 {
            assert_eq!(s.tableau[i].len(), i + 1, "tableau pile {i} size");
        }
        assert_eq!(s.stock.len(), 52 - 28);
        assert!(s.waste.is_empty());
        assert!(s.foundation.iter().all(|p| p.is_empty()));
        // Only the last card of each pile is face-up.
        for pile in &s.tableau {
            for (i, c) in pile.iter().enumerate() {
                let last = i == pile.len() - 1;
                assert_eq!(c.face_up, last);
            }
        }
    }

    #[test]
    fn deal_is_deterministic() {
        let a = GameState::new(&klondike(), 12345);
        let b = GameState::new(&klondike(), 12345);
        assert_eq!(format!("{:?}", a.tableau), format!("{:?}", b.tableau));
        assert_eq!(format!("{:?}", a.stock), format!("{:?}", b.stock));
    }

    #[test]
    fn different_seeds_differ() {
        let a = GameState::new(&klondike(), 1);
        let b = GameState::new(&klondike(), 2);
        assert_ne!(format!("{:?}", a.stock), format!("{:?}", b.stock));
    }

    #[test]
    fn total_cards_is_52() {
        let s = GameState::new(&klondike(), 7);
        assert_eq!(s.total_cards(), 52);
    }
}
