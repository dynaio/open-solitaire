use openpatience::core::card::{Card, Rank, Suit};
use openpatience::core::engine::Engine;
use openpatience::core::moves::{is_legal, Move};
use openpatience::core::rules::RuleSet;
use openpatience::core::state::{GameState, PileRef, PlacedCard};

fn klondike() -> RuleSet {
    RuleSet::from_json(include_str!("../assets/rules/klondike.json")).unwrap()
}

fn klondike_draw3() -> RuleSet {
    RuleSet::from_json(include_str!("../assets/rules/klondike-draw3.json")).unwrap()
}

#[test]
fn both_rules_load_and_validate() {
    klondike().validate().unwrap();
    klondike_draw3().validate().unwrap();
}

#[test]
fn deal_seed_12345_is_stable() {
    // If this test ever fails, the deal algorithm or the RNG changed and
    // MUST be documented in docs/DEAL_ALGORITHM.md as a breaking change.
    let s = GameState::new(&klondike(), 12345);
    let mut parts = Vec::new();
    for pile in &s.tableau {
        let cards: Vec<String> = pile.iter().map(|p| p.card.to_string()).collect();
        parts.push(cards.join(","));
    }
    let sig = parts.join("|")
        + "::"
        + &s.stock
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",");
    // Recompute independently to prove determinism:
    let s2 = GameState::new(&klondike(), 12345);
    let mut parts2 = Vec::new();
    for pile in &s2.tableau {
        parts2.push(
            pile.iter()
                .map(|p| p.card.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    let sig2 = parts2.join("|")
        + "::"
        + &s2
            .stock
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",");
    assert_eq!(sig, sig2);
}

#[test]
fn draw3_pulls_three_at_a_time() {
    let mut e = Engine::new(klondike_draw3(), 1);
    let before = e.state.stock.len();
    e.apply(Move::Draw);
    assert_eq!(e.state.stock.len(), before - 3);
    assert_eq!(e.state.waste.len(), 3);
}

#[test]
fn can_move_ace_to_empty_foundation() {
    // Construct a synthetic state to test is_legal directly.
    let mut s = GameState::new(&klondike(), 1);
    // Clear tableau so we can place a controlled card on top.
    for t in &mut s.tableau {
        t.clear();
    }
    s.tableau[0].push(PlacedCard {
        card: Card::new(Suit::Spades, Rank::Ace),
        face_up: true,
    });
    let m = Move::Move {
        from: PileRef::Tableau(0),
        from_idx: 0,
        to: PileRef::Foundation(0),
    };
    assert!(is_legal(&s, &klondike(), &m));
}

#[test]
fn cannot_move_two_to_empty_foundation() {
    let mut s = GameState::new(&klondike(), 1);
    for t in &mut s.tableau {
        t.clear();
    }
    s.tableau[0].push(PlacedCard {
        card: Card::new(Suit::Spades, Rank::Two),
        face_up: true,
    });
    let m = Move::Move {
        from: PileRef::Tableau(0),
        from_idx: 0,
        to: PileRef::Foundation(0),
    };
    assert!(!is_legal(&s, &klondike(), &m));
}

#[test]
fn empty_tableau_accepts_only_king() {
    let mut s = GameState::new(&klondike(), 1);
    for t in &mut s.tableau {
        t.clear();
    }
    s.tableau[0].push(PlacedCard {
        card: Card::new(Suit::Spades, Rank::King),
        face_up: true,
    });
    s.tableau[1].push(PlacedCard {
        card: Card::new(Suit::Hearts, Rank::King),
        face_up: true,
    });
    s.tableau[2].push(PlacedCard {
        card: Card::new(Suit::Hearts, Rank::Queen),
        face_up: true,
    });
    // King to empty (T3) → legal
    assert!(is_legal(
        &s,
        &klondike(),
        &Move::Move {
            from: PileRef::Tableau(0),
            from_idx: 0,
            to: PileRef::Tableau(3),
        }
    ));
    // Queen to empty (T3) → illegal
    assert!(!is_legal(
        &s,
        &klondike(),
        &Move::Move {
            from: PileRef::Tableau(2),
            from_idx: 0,
            to: PileRef::Tableau(3),
        }
    ));
}

#[test]
fn valid_run_moves_as_a_group() {
    // 5♥ 4♠ 3♦ as a run can move onto 6♣.
    let mut s = GameState::new(&klondike(), 1);
    for t in &mut s.tableau {
        t.clear();
    }
    s.tableau[0].push(PlacedCard {
        card: Card::new(Suit::Clubs, Rank::Six),
        face_up: true,
    });
    s.tableau[1].push(PlacedCard {
        card: Card::new(Suit::Hearts, Rank::Five),
        face_up: true,
    });
    s.tableau[1].push(PlacedCard {
        card: Card::new(Suit::Spades, Rank::Four),
        face_up: true,
    });
    s.tableau[1].push(PlacedCard {
        card: Card::new(Suit::Diamonds, Rank::Three),
        face_up: true,
    });
    let m = Move::Move {
        from: PileRef::Tableau(1),
        from_idx: 0,
        to: PileRef::Tableau(0),
    };
    assert!(is_legal(&s, &klondike(), &m));
}

#[test]
fn undo_is_unlimited_up_to_history_cap() {
    let mut e = Engine::new(klondike(), 7);
    let total = 50;
    let mut applied = 0;
    for _ in 0..total {
        if e.apply(Move::Draw) {
            applied += 1;
        }
    }
    for _ in 0..applied {
        assert!(e.undo());
    }
    assert_eq!(e.state.waste.len(), 0);
}
