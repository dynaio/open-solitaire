use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSet {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub decks: u8,
    #[serde(default)]
    pub scoring: Scoring,
    pub tableau: TableauRule,
    pub foundation: FoundationRule,
    pub stock: StockRule,
    #[serde(default)]
    pub free_cells: u8,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Scoring {
    #[default] Standard,
    Vegas,
    Timed,
    None,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TableauRule {
    pub piles: u8,
    pub initial_deal_pattern: Vec<u8>,
    pub only_last_face_up: bool,
    pub build: BuildOrder,
    pub alternate_color: bool,
    pub empty_pile_accepts: EmptyPile,
    #[serde(default)]
    pub move_group: MoveGroup,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildOrder { Ascending, Descending }

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmptyPile { Any, King, Ace, None }

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MoveGroup {
    #[default] Any,
    OnlyFaceUp,
    OnlySingle,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationRule {
    pub piles: u8,
    pub build: BuildOrder,
    pub same_suit: bool,
    pub start_rank: StartRank,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StartRank { Ace, King, Two }

impl StartRank {
    pub fn as_value(self) -> u8 {
        match self { StartRank::Ace => 1, StartRank::Two => 2, StartRank::King => 13 }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StockRule {
    pub draw: u8,
    pub redeals: Redeals,
}

/// Redials limit: `"infinite"` or a non-negative integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redeals {
    Infinite,
    Count(u32),
}

impl<'de> Deserialize<'de> for Redeals {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Redeals;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "'infinite' or a non-negative integer")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Redeals, E> {
                if v == "infinite" { Ok(Redeals::Infinite) }
                else { Err(E::custom(format!("expected 'infinite', got '{}'", v))) }
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Redeals, E> {
                Ok(Redeals::Count(v as u32))
            }
        }
        d.deserialize_any(V)
    }
}

impl Serialize for Redeals {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Redeals::Infinite => s.serialize_str("infinite"),
            Redeals::Count(n) => s.serialize_u32(*n),
        }
    }
}

impl RuleSet {
    pub fn from_json(s: &str) -> Result<RuleSet, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Total cards required by the rule (decks × 52).
    pub fn total_cards(&self) -> usize { self.decks as usize * 52 }

    /// Validate the rule is internally consistent.
    pub fn validate(&self) -> Result<(), String> {
        if self.tableau.initial_deal_pattern.len() != self.tableau.piles as usize {
            return Err(format!(
                "initial_deal_pattern has {} entries but tableau.piles = {}",
                self.tableau.initial_deal_pattern.len(), self.tableau.piles
            ));
        }
        let dealt: u32 = self.tableau.initial_deal_pattern.iter().map(|&n| n as u32).sum();
        if dealt as usize > self.total_cards() {
            return Err(format!(
                "deal pattern needs {} cards but only {} available",
                dealt, self.total_cards()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn klondike_parses() {
        let json = std::fs::read_to_string("assets/rules/klondike.json").unwrap();
        let r = RuleSet::from_json(&json).unwrap();
        assert_eq!(r.id, "klondike");
        assert_eq!(r.tableau.piles, 7);
        r.validate().unwrap();
    }

    #[test]
    fn redeals_infinite_parses() {
        let r = RuleSet::from_json(&std::fs::read_to_string("assets/rules/klondike.json").unwrap()).unwrap();
        assert_eq!(r.stock.redeals, Redeals::Infinite);
    }

    #[test]
    fn unknown_field_rejected() {
        let json = r#"{"id":"x","name":"x","decks":1,
            "tableau":{"piles":1,"initial_deal_pattern":[1],"only_last_face_up":true,
            "build":"descending","alternate_color":true,"empty_pile_accepts":"king"},
            "foundation":{"piles":1,"build":"ascending","same_suit":true,"start_rank":"ace"},
            "stock":{"draw":1,"redeals":1},
            "typo_field":true}"#;
        assert!(RuleSet::from_json(json).is_err());
    }
}
