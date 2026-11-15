use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Suit { Spades, Hearts, Diamonds, Clubs }

impl Suit {
    pub fn is_red(self) -> bool { matches!(self, Suit::Hearts | Suit::Diamonds) }
    pub fn glyph(self) -> char {
        match self {
            Suit::Spades => '\u{2660}', Suit::Hearts => '\u{2665}',
            Suit::Diamonds => '\u{2666}', Suit::Clubs => '\u{2663}',
        }
    }
    pub fn as_u8(self) -> u8 {
        match self { Suit::Spades => 0, Suit::Hearts => 1, Suit::Diamonds => 2, Suit::Clubs => 3 }
    }
    pub fn all() -> [Suit; 4] { [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs] }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rank {
    Ace, Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King,
}

impl Rank {
    pub fn value(self) -> u8 {
        match self {
            Rank::Ace => 1, Rank::Two => 2, Rank::Three => 3, Rank::Four => 4,
            Rank::Five => 5, Rank::Six => 6, Rank::Seven => 7, Rank::Eight => 8,
            Rank::Nine => 9, Rank::Ten => 10, Rank::Jack => 11, Rank::Queen => 12, Rank::King => 13,
        }
    }
    pub fn glyph(self) -> &'static str {
        match self {
            Rank::Ace => "A", Rank::Two => "2", Rank::Three => "3", Rank::Four => "4",
            Rank::Five => "5", Rank::Six => "6", Rank::Seven => "7", Rank::Eight => "8",
            Rank::Nine => "9", Rank::Ten => "10", Rank::Jack => "J", Rank::Queen => "Q", Rank::King => "K",
        }
    }
    pub fn all() -> [Rank; 13] {
        use Rank::*;
        [Ace, Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card { pub suit: Suit, pub rank: Rank }

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self { Card { suit, rank } }

    /// Deterministic ID in `0..=51`. Encodes suit-major, rank-minor.
    pub fn id(self) -> u8 { self.suit.as_u8() * 13 + (self.rank.value() - 1) }

    pub fn full_deck() -> Vec<Card> {
        let mut v = Vec::with_capacity(52);
        for s in Suit::all() { for r in Rank::all() { v.push(Card::new(s, r)); } }
        v
    }
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.rank.glyph(), self.suit.glyph())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_has_52_unique_cards() {
        let deck = Card::full_deck();
        assert_eq!(deck.len(), 52);
        let mut ids: Vec<u8> = deck.iter().map(|c| c.id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 52);
    }

    #[test]
    fn suit_redness() {
        assert!(Suit::Hearts.is_red());
        assert!(Suit::Diamonds.is_red());
        assert!(!Suit::Spades.is_red());
        assert!(!Suit::Clubs.is_red());
    }
}
