use gtk4::cairo;
use openpatience::core::card::Suit;

#[derive(Clone, Copy, Debug)]
pub struct Rgb(pub f64, pub f64, pub f64);

impl Rgb {
    pub fn set_source(self, cr: &cairo::Context) {
        cr.set_source_rgb(self.0, self.1, self.2);
    }
    pub fn set_source_alpha(self, cr: &cairo::Context, a: f64) {
        cr.set_source_rgba(self.0, self.1, self.2, a);
    }
}

pub struct Theme {
    pub table_bg: Rgb,
    pub card_face_bg: Rgb,
    pub card_face_border: Rgb,
    pub card_back_color: Rgb,
    pub card_back_pattern: Rgb,
    pub suit_red: Rgb,
    pub suit_black: Rgb,
    pub empty_slot: Rgb,
    pub highlight: Rgb,
}

impl Theme {
    pub fn modern_felt() -> Self {
        Theme {
            table_bg: Rgb(0.051, 0.298, 0.180),
            card_face_bg: Rgb(0.985, 0.985, 0.985),
            card_face_border: Rgb(0.820, 0.820, 0.830),
            card_back_color: Rgb(0.118, 0.227, 0.373),
            card_back_pattern: Rgb(0.612, 0.706, 0.831),
            // Real-world playing-card red and black
            suit_red: Rgb(0.860, 0.115, 0.130),
            suit_black: Rgb(0.080, 0.080, 0.100),
            empty_slot: Rgb(1.0, 1.0, 1.0),
            highlight: Rgb(0.298, 0.686, 0.314),
        }
    }

    pub fn suit_color(&self, suit: Suit) -> Rgb {
        if suit.is_red() { self.suit_red } else { self.suit_black }
    }
}
