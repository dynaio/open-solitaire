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
    #[allow(dead_code)]
    pub fn lighten(self, amt: f64) -> Self {
        Rgb(
            (self.0 + amt).min(1.0),
            (self.1 + amt).min(1.0),
            (self.2 + amt).min(1.0),
        )
    }
    #[allow(dead_code)]
    pub fn darken(self, amt: f64) -> Self {
        Rgb(
            (self.0 - amt).max(0.0),
            (self.1 - amt).max(0.0),
            (self.2 - amt).max(0.0),
        )
    }
}

pub struct Theme {
    pub table_bg: Rgb,
    pub table_bg_center: Rgb, // for radial gradient
    pub card_face_bg: Rgb,
    pub card_face_border: Rgb,
    pub card_back_color: Rgb,
    pub card_back_pattern: Rgb,
    pub suit_red: Rgb,
    pub suit_black: Rgb,
    pub empty_slot: Rgb,
    pub highlight: Rgb,
    pub gold: Rgb,
}

impl Theme {
    pub fn deep_emerald() -> Self {
        Theme {
            table_bg: Rgb(0.055, 0.260, 0.150),
            table_bg_center: Rgb(0.090, 0.340, 0.200),
            card_face_bg: Rgb(0.995, 0.993, 0.985),
            card_face_border: Rgb(0.700, 0.700, 0.720),
            card_back_color: Rgb(0.085, 0.140, 0.290),
            card_back_pattern: Rgb(0.855, 0.730, 0.360),
            suit_red: Rgb(0.870, 0.095, 0.130),
            suit_black: Rgb(0.055, 0.055, 0.075),
            empty_slot: Rgb(1.0, 1.0, 1.0),
            highlight: Rgb(0.980, 0.820, 0.280),
            gold: Rgb(0.900, 0.755, 0.360),
        }
    }

    pub fn suit_color(&self, suit: Suit) -> Rgb {
        if suit.is_red() {
            self.suit_red
        } else {
            self.suit_black
        }
    }

    #[allow(dead_code)]
    pub fn midnight_blue() -> Self {
        Theme {
            table_bg: Rgb(0.035, 0.070, 0.180),
            table_bg_center: Rgb(0.060, 0.110, 0.240),
            card_face_bg: Rgb(0.995, 0.993, 0.985),
            card_face_border: Rgb(0.700, 0.700, 0.720),
            card_back_color: Rgb(0.180, 0.030, 0.090),
            card_back_pattern: Rgb(0.980, 0.830, 0.420),
            suit_red: Rgb(0.900, 0.100, 0.130),
            suit_black: Rgb(0.040, 0.040, 0.060),
            empty_slot: Rgb(1.0, 1.0, 1.0),
            highlight: Rgb(0.980, 0.820, 0.280),
            gold: Rgb(0.900, 0.755, 0.360),
        }
    }

    #[allow(dead_code)]
    pub fn crimson() -> Self {
        Theme {
            table_bg: Rgb(0.200, 0.030, 0.040),
            table_bg_center: Rgb(0.280, 0.050, 0.060),
            card_face_bg: Rgb(0.998, 0.996, 0.990),
            card_face_border: Rgb(0.680, 0.680, 0.700),
            card_back_color: Rgb(0.040, 0.100, 0.260),
            card_back_pattern: Rgb(0.950, 0.850, 0.350),
            suit_red: Rgb(0.850, 0.080, 0.100),
            suit_black: Rgb(0.040, 0.040, 0.060),
            empty_slot: Rgb(1.0, 1.0, 1.0),
            highlight: Rgb(0.980, 0.820, 0.280),
            gold: Rgb(0.880, 0.730, 0.330),
        }
    }
}
