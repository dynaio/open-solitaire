use gtk4::cairo;
use openpatience::core::card::{Card, Rank, Suit};

use super::theme::{Rgb, Theme};

pub const CARD_W: f64 = 100.0;
pub const CARD_H: f64 = 140.0;
pub const CARD_RADIUS: f64 = 8.0;

// ─────────────────────────────────────────────────────────────────
// Shape primitives
// ─────────────────────────────────────────────────────────────────

pub fn rounded_rect(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
    cr.close_path();
}

/// Draw a filled suit shape centered at (cx, cy) with half-height `s`.
pub fn draw_suit_shape(cr: &cairo::Context, suit: Suit, cx: f64, cy: f64, s: f64) {
    match suit {
        Suit::Hearts => draw_heart(cr, cx, cy, s),
        Suit::Diamonds => draw_diamond(cr, cx, cy, s),
        Suit::Spades => draw_spade(cr, cx, cy, s),
        Suit::Clubs => draw_club(cr, cx, cy, s),
    }
    cr.fill().ok();
}

fn draw_heart(cr: &cairo::Context, cx: f64, cy: f64, s: f64) {
    let w = s * 0.95;
    cr.move_to(cx, cy + s);
    cr.curve_to(
        cx - w * 1.15, cy + s * 0.15,
        cx - w * 1.05, cy - s * 0.85,
        cx - w * 0.35, cy - s * 0.85,
    );
    cr.curve_to(
        cx - w * 0.10, cy - s * 0.85,
        cx, cy - s * 0.45,
        cx, cy - s * 0.35,
    );
    cr.curve_to(
        cx, cy - s * 0.45,
        cx + w * 0.10, cy - s * 0.85,
        cx + w * 0.35, cy - s * 0.85,
    );
    cr.curve_to(
        cx + w * 1.05, cy - s * 0.85,
        cx + w * 1.15, cy + s * 0.15,
        cx, cy + s,
    );
    cr.close_path();
}

fn draw_diamond(cr: &cairo::Context, cx: f64, cy: f64, s: f64) {
    let w = s * 0.68;
    cr.move_to(cx, cy - s);
    cr.line_to(cx + w, cy);
    cr.line_to(cx, cy + s);
    cr.line_to(cx - w, cy);
    cr.close_path();
}

fn draw_spade(cr: &cairo::Context, cx: f64, cy: f64, s: f64) {
    let w = s * 0.95;
    // Body (upside-down heart)
    cr.move_to(cx, cy - s);
    cr.curve_to(
        cx + w * 1.15, cy - s * 0.15,
        cx + w * 1.05, cy + s * 0.85,
        cx + w * 0.35, cy + s * 0.85,
    );
    cr.curve_to(
        cx + w * 0.10, cy + s * 0.85,
        cx, cy + s * 0.45,
        cx, cy + s * 0.35,
    );
    cr.curve_to(
        cx, cy + s * 0.45,
        cx - w * 0.10, cy + s * 0.85,
        cx - w * 0.35, cy + s * 0.85,
    );
    cr.curve_to(
        cx - w * 1.05, cy + s * 0.85,
        cx - w * 1.15, cy - s * 0.15,
        cx, cy - s,
    );
    cr.close_path();
    // Stem
    cr.move_to(cx - s * 0.15, cy + s);
    cr.line_to(cx - s * 0.10, cy + s * 0.7);
    cr.line_to(cx + s * 0.10, cy + s * 0.7);
    cr.line_to(cx + s * 0.15, cy + s);
    cr.close_path();
}

fn draw_club(cr: &cairo::Context, cx: f64, cy: f64, s: f64) {
    let r = s * 0.36;
    let lobe = s * 0.42;
    // Top circle
    cr.arc(cx, cy - lobe, r, 0.0, 2.0 * std::f64::consts::PI);
    cr.close_path();
    // Left circle
    cr.arc(cx - r * 0.95, cy + lobe * 0.3, r, 0.0, 2.0 * std::f64::consts::PI);
    cr.close_path();
    // Right circle
    cr.arc(cx + r * 0.95, cy + lobe * 0.3, r, 0.0, 2.0 * std::f64::consts::PI);
    cr.close_path();
    // Stem
    cr.move_to(cx - s * 0.14, cy + s);
    cr.line_to(cx - s * 0.08, cy + s * 0.55);
    cr.line_to(cx + s * 0.08, cy + s * 0.55);
    cr.line_to(cx + s * 0.14, cy + s);
    cr.close_path();
}

// ─────────────────────────────────────────────────────────────────
// Pip layouts
// ─────────────────────────────────────────────────────────────────

fn pip_positions(rank: Rank) -> Vec<(f64, f64)> {
    use Rank::*;
    let l = 0.30;
    let c = 0.50;
    let r = 0.70;
    match rank {
        Two   => vec![(c, 0.22), (c, 0.78)],
        Three => vec![(c, 0.22), (c, 0.50), (c, 0.78)],
        Four  => vec![(l, 0.22), (r, 0.22), (l, 0.78), (r, 0.78)],
        Five  => vec![(l, 0.22), (r, 0.22), (c, 0.50), (l, 0.78), (r, 0.78)],
        Six   => vec![(l, 0.22), (r, 0.22), (l, 0.50), (r, 0.50), (l, 0.78), (r, 0.78)],
        Seven => vec![(l, 0.22), (r, 0.22), (c, 0.36), (l, 0.50), (r, 0.50), (l, 0.78), (r, 0.78)],
        Eight => vec![(l, 0.22), (r, 0.22), (c, 0.36), (l, 0.50), (r, 0.50), (c, 0.64), (l, 0.78), (r, 0.78)],
        Nine  => vec![(l, 0.18), (r, 0.18), (l, 0.36), (r, 0.36), (c, 0.50),
                      (l, 0.64), (r, 0.64), (l, 0.82), (r, 0.82)],
        Ten   => vec![(l, 0.18), (r, 0.18), (c, 0.30), (l, 0.42), (r, 0.42),
                      (l, 0.58), (r, 0.58), (c, 0.70), (l, 0.82), (r, 0.82)],
        _ => vec![],
    }
}

// ─────────────────────────────────────────────────────────────────
// Card face
// ─────────────────────────────────────────────────────────────────

pub fn draw_card_face(
    cr: &cairo::Context,
    x: f64, y: f64, w: f64, h: f64,
    card: Card, theme: &Theme,
) {
    // Shadow
    rounded_rect(cr, x + 2.0, y + 3.0, w, h, CARD_RADIUS);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.22);
    cr.fill().ok();

    // Face
    rounded_rect(cr, x, y, w, h, CARD_RADIUS);
    theme.card_face_bg.set_source(cr);
    cr.fill_preserve().ok();
    cr.set_line_width(1.0);
    theme.card_face_border.set_source(cr);
    cr.stroke().ok();

    let sc = theme.suit_color(card.suit);

    // --- Corner indices ---
    draw_corner_index(cr, x, y, w, h, card, sc, 1.0);
    draw_corner_index(cr, x, y, w, h, card, sc, -1.0);

    // --- Center ---
    draw_center(cr, x, y, w, h, card, sc, theme);
}

fn draw_corner_index(
    cr: &cairo::Context,
    x: f64, y: f64, w: f64, h: f64,
    card: Card, sc: Rgb, orientation: f64,
) {
    cr.save().ok();
    if orientation < 0.0 {
        cr.translate(x + w, y + h);
        cr.rotate(std::f64::consts::PI);
        cr.translate(-x, -y);
    }

    // Rank
    sc.set_source(cr);
    cr.select_font_face("Noto Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(w * 0.20);
    let rank_str = card.rank.glyph();
    if let Ok(ext) = cr.text_extents(rank_str) {
        cr.move_to(
            x + w * 0.05 + (w * 0.18 - ext.width()) / 2.0 - ext.x_bearing(),
            y + h * 0.13,
        );
    }
    cr.show_text(rank_str).ok();

    // Suit
    let suit_size = w * 0.075;
    draw_suit_shape(cr, card.suit, x + w * 0.14, y + h * 0.22 + suit_size, suit_size);
    cr.restore().ok();
}

fn draw_center(
    cr: &cairo::Context,
    x: f64, y: f64, w: f64, h: f64,
    card: Card, sc: Rgb, theme: &Theme,
) {
    sc.set_source(cr);
    match card.rank {
        Rank::Ace => {
            // Large centered suit
            draw_suit_shape(cr, card.suit, x + w / 2.0, y + h / 2.0, w * 0.24);
        }
        Rank::Jack | Rank::Queen | Rank::King => {
            draw_face_card(cr, x, y, w, h, card, sc, theme);
        }
        _ => {
            // Pip layout
            let pip_size = w * 0.075;
            for (px, py) in pip_positions(card.rank) {
                draw_suit_shape(cr, card.suit, x + px * w, y + py * h, pip_size);
            }
        }
    }
}

fn draw_face_card(
    cr: &cairo::Context,
    x: f64, y: f64, w: f64, h: f64,
    card: Card, sc: Rgb, theme: &Theme,
) {
    // Decorative inner frame
    let inset_x = w * 0.20;
    let inset_y = h * 0.22;
    let inner = (x + inset_x, y + inset_y, w - inset_x * 2.0, h - inset_y * 2.0);

    // Fill inner rectangle with a very light tint
    theme.card_face_border.set_source_alpha(cr, 0.35);
    cr.rectangle(inner.0, inner.1, inner.2, inner.3);
    cr.fill().ok();

    // Border
    sc.set_source_alpha(cr, 0.75);
    cr.set_line_width(1.2);
    cr.rectangle(inner.0, inner.1, inner.2, inner.3);
    cr.stroke().ok();

    // Letter in the middle (no extra suits — corners already show the suit)
    sc.set_source(cr);
    cr.select_font_face("Noto Serif", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(w * 0.38);
    let letter = card.rank.glyph();
    if let Ok(ext) = cr.text_extents(letter) {
        cr.move_to(
            x + w / 2.0 - ext.width() / 2.0 - ext.x_bearing(),
            y + h / 2.0 - ext.height() / 2.0 - ext.y_bearing(),
        );
    }
    cr.show_text(letter).ok();
}

// ─────────────────────────────────────────────────────────────────
// Back and slot
// ─────────────────────────────────────────────────────────────────

pub fn draw_card_back(
    cr: &cairo::Context,
    x: f64, y: f64, w: f64, h: f64,
    theme: &Theme,
) {
    rounded_rect(cr, x + 2.0, y + 3.0, w, h, CARD_RADIUS);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.22);
    cr.fill().ok();

    rounded_rect(cr, x, y, w, h, CARD_RADIUS);
    theme.card_back_color.set_source(cr);
    cr.fill_preserve().ok();
    cr.set_line_width(1.0);
    theme.card_face_border.set_source(cr);
    cr.stroke().ok();

    let inset = w * 0.08;
    rounded_rect(cr, x + inset, y + inset, w - 2.0 * inset, h - 2.0 * inset, CARD_RADIUS * 0.7);
    cr.set_line_width(1.5);
    theme.card_back_pattern.set_source_alpha(cr, 0.55);
    cr.stroke().ok();

    // Center diamond emblem (like real cards)
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let dr = w * 0.14;
    cr.move_to(cx, cy - dr);
    cr.line_to(cx + dr, cy);
    cr.line_to(cx, cy + dr);
    cr.line_to(cx - dr, cy);
    cr.close_path();
    theme.card_back_pattern.set_source_alpha(cr, 0.40);
    cr.fill().ok();
}

pub fn draw_empty_slot(
    cr: &cairo::Context,
    x: f64, y: f64, w: f64, h: f64,
    theme: &Theme,
) {
    rounded_rect(cr, x, y, w, h, CARD_RADIUS);
    theme.empty_slot.set_source_alpha(cr, 0.10);
    cr.fill_preserve().ok();
    cr.set_line_width(1.5);
    cr.set_dash(&[4.0, 4.0], 0.0);
    theme.empty_slot.set_source_alpha(cr, 0.42);
    cr.stroke().ok();
    cr.set_dash(&[], 0.0);
}

pub fn draw_selected_highlight(
    cr: &cairo::Context,
    x: f64, y: f64, w: f64, h: f64,
    theme: &Theme,
) {
    rounded_rect(cr, x - 2.0, y - 2.0, w + 4.0, h + 4.0, CARD_RADIUS + 2.0);
    cr.set_line_width(3.0);
    theme.highlight.set_source(cr);
    cr.stroke().ok();
}

pub fn draw_hover_highlight(
    cr: &cairo::Context,
    x: f64, y: f64, w: f64, h: f64,
    theme: &Theme,
) {
    rounded_rect(cr, x - 1.0, y - 1.0, w + 2.0, h + 2.0, CARD_RADIUS + 1.0);
    cr.set_line_width(2.0);
    theme.highlight.set_source_alpha(cr, 0.5);
    cr.stroke().ok();
}
