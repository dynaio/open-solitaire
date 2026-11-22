use gtk4::cairo;
use openpatience::core::card::{Card, Rank, Suit};

use super::theme::{Rgb, Theme};

pub const CARD_W: f64 = 100.0;
pub const CARD_H: f64 = 140.0;
pub const CARD_RADIUS: f64 = 9.0;

// ─────────────────────────────────────────────────────────────────
// Shape primitives
// ─────────────────────────────────────────────────────────────────

pub fn rounded_rect(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(
        x + r,
        y + h - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.arc(
        x + r,
        y + r,
        r,
        std::f64::consts::PI,
        3.0 * std::f64::consts::FRAC_PI_2,
    );
    cr.close_path();
}

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
        cx - w * 1.15,
        cy + s * 0.15,
        cx - w * 1.05,
        cy - s * 0.85,
        cx - w * 0.35,
        cy - s * 0.85,
    );
    cr.curve_to(
        cx - w * 0.10,
        cy - s * 0.85,
        cx,
        cy - s * 0.45,
        cx,
        cy - s * 0.35,
    );
    cr.curve_to(
        cx,
        cy - s * 0.45,
        cx + w * 0.10,
        cy - s * 0.85,
        cx + w * 0.35,
        cy - s * 0.85,
    );
    cr.curve_to(
        cx + w * 1.05,
        cy - s * 0.85,
        cx + w * 1.15,
        cy + s * 0.15,
        cx,
        cy + s,
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
    cr.move_to(cx, cy - s);
    cr.curve_to(
        cx + w * 1.15,
        cy - s * 0.15,
        cx + w * 1.05,
        cy + s * 0.85,
        cx + w * 0.35,
        cy + s * 0.85,
    );
    cr.curve_to(
        cx + w * 0.10,
        cy + s * 0.85,
        cx,
        cy + s * 0.45,
        cx,
        cy + s * 0.35,
    );
    cr.curve_to(
        cx,
        cy + s * 0.45,
        cx - w * 0.10,
        cy + s * 0.85,
        cx - w * 0.35,
        cy + s * 0.85,
    );
    cr.curve_to(
        cx - w * 1.05,
        cy + s * 0.85,
        cx - w * 1.15,
        cy - s * 0.15,
        cx,
        cy - s,
    );
    cr.close_path();
    cr.move_to(cx - s * 0.15, cy + s);
    cr.line_to(cx - s * 0.10, cy + s * 0.7);
    cr.line_to(cx + s * 0.10, cy + s * 0.7);
    cr.line_to(cx + s * 0.15, cy + s);
    cr.close_path();
}

fn draw_club(cr: &cairo::Context, cx: f64, cy: f64, s: f64) {
    let r = s * 0.36;
    let lobe = s * 0.42;
    cr.arc(cx, cy - lobe, r, 0.0, 2.0 * std::f64::consts::PI);
    cr.close_path();
    cr.arc(
        cx - r * 0.95,
        cy + lobe * 0.3,
        r,
        0.0,
        2.0 * std::f64::consts::PI,
    );
    cr.close_path();
    cr.arc(
        cx + r * 0.95,
        cy + lobe * 0.3,
        r,
        0.0,
        2.0 * std::f64::consts::PI,
    );
    cr.close_path();
    cr.move_to(cx - s * 0.14, cy + s);
    cr.line_to(cx - s * 0.08, cy + s * 0.55);
    cr.line_to(cx + s * 0.08, cy + s * 0.55);
    cr.line_to(cx + s * 0.14, cy + s);
    cr.close_path();
}

// ─────────────────────────────────────────────────────────────────
// Pip layouts (slightly larger than before)
// ─────────────────────────────────────────────────────────────────

fn pip_positions(rank: Rank) -> Vec<(f64, f64)> {
    use Rank::*;
    let l = 0.30;
    let c = 0.50;
    let r = 0.70;
    match rank {
        Two => vec![(c, 0.20), (c, 0.80)],
        Three => vec![(c, 0.20), (c, 0.50), (c, 0.80)],
        Four => vec![(l, 0.20), (r, 0.20), (l, 0.80), (r, 0.80)],
        Five => vec![(l, 0.20), (r, 0.20), (c, 0.50), (l, 0.80), (r, 0.80)],
        Six => vec![
            (l, 0.20),
            (r, 0.20),
            (l, 0.50),
            (r, 0.50),
            (l, 0.80),
            (r, 0.80),
        ],
        Seven => vec![
            (l, 0.20),
            (r, 0.20),
            (c, 0.35),
            (l, 0.50),
            (r, 0.50),
            (l, 0.80),
            (r, 0.80),
        ],
        Eight => vec![
            (l, 0.20),
            (r, 0.20),
            (c, 0.35),
            (l, 0.50),
            (r, 0.50),
            (c, 0.65),
            (l, 0.80),
            (r, 0.80),
        ],
        Nine => vec![
            (l, 0.16),
            (r, 0.16),
            (l, 0.34),
            (r, 0.34),
            (c, 0.50),
            (l, 0.66),
            (r, 0.66),
            (l, 0.84),
            (r, 0.84),
        ],
        Ten => vec![
            (l, 0.16),
            (r, 0.16),
            (c, 0.28),
            (l, 0.40),
            (r, 0.40),
            (l, 0.60),
            (r, 0.60),
            (c, 0.72),
            (l, 0.84),
            (r, 0.84),
        ],
        _ => vec![],
    }
}

// ─────────────────────────────────────────────────────────────────
// Card face
// ─────────────────────────────────────────────────────────────────

pub fn draw_card_face(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    card: Card,
    theme: &Theme,
) {
    // Two-layer shadow for depth
    rounded_rect(cr, x + 3.0, y + 4.0, w, h, CARD_RADIUS);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.14);
    cr.fill().ok();
    rounded_rect(cr, x + 1.5, y + 2.0, w, h, CARD_RADIUS);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.22);
    cr.fill().ok();

    // Face with slight inner gradient
    rounded_rect(cr, x, y, w, h, CARD_RADIUS);
    theme.card_face_bg.set_source(cr);
    cr.fill_preserve().ok();
    cr.set_line_width(1.0);
    theme.card_face_border.set_source(cr);
    cr.stroke().ok();

    let sc = theme.suit_color(card.suit);

    // Corner indices (bigger, bolder)
    draw_corner_index(cr, x, y, w, h, card, sc, 1.0);
    draw_corner_index(cr, x, y, w, h, card, sc, -1.0);

    // Center
    draw_center(cr, x, y, w, h, card, sc, theme);
}

fn draw_corner_index(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    card: Card,
    sc: Rgb,
    orientation: f64,
) {
    cr.save().ok();
    if orientation < 0.0 {
        cr.translate(x + w, y + h);
        cr.rotate(std::f64::consts::PI);
        cr.translate(-x, -y);
    }

    // Bigger rank text
    sc.set_source(cr);
    cr.select_font_face(
        "Noto Sans",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(w * 0.235);
    let rank_str = card.rank.glyph();
    if let Ok(ext) = cr.text_extents(rank_str) {
        let cx = x + w * 0.14;
        cr.move_to(cx - ext.width() / 2.0 - ext.x_bearing(), y + h * 0.155);
    }
    cr.show_text(rank_str).ok();

    // Suit under the rank
    let suit_size = w * 0.075;
    draw_suit_shape(
        cr,
        card.suit,
        x + w * 0.14,
        y + h * 0.255 + suit_size,
        suit_size,
    );
    cr.restore().ok();
}

fn draw_center(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    card: Card,
    sc: Rgb,
    theme: &Theme,
) {
    sc.set_source(cr);
    match card.rank {
        Rank::Ace => {
            draw_suit_shape(cr, card.suit, x + w / 2.0, y + h / 2.0, w * 0.26);
        }
        Rank::Jack | Rank::Queen | Rank::King => {
            draw_face_card(cr, x, y, w, h, card, sc, theme);
        }
        _ => {
            let pip_size = w * 0.080;
            for (px, py) in pip_positions(card.rank) {
                draw_suit_shape(cr, card.suit, x + px * w, y + py * h, pip_size);
            }
        }
    }
}

fn draw_face_card(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    card: Card,
    sc: Rgb,
    theme: &Theme,
) {
    // Inner frame
    let ix = w * 0.22;
    let iy = h * 0.24;
    let (fx, fy, fw, fh) = (x + ix, y + iy, w - ix * 2.0, h - iy * 2.0);

    // Soft fill
    theme.card_face_border.set_source_alpha(cr, 0.30);
    cr.rectangle(fx, fy, fw, fh);
    cr.fill().ok();

    // Faint suit watermark behind the letter
    let wm_size = fw.min(fh) * 0.55;
    sc.set_source_alpha(cr, 0.08);
    draw_suit_shape(cr, card.suit, fx + fw / 2.0, fy + fh / 2.0, wm_size);

    // Border
    sc.set_source_alpha(cr, 0.80);
    cr.set_line_width(1.3);
    cr.rectangle(fx, fy, fw, fh);
    cr.stroke().ok();

    // Big letter
    sc.set_source(cr);
    cr.select_font_face(
        "Noto Serif",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(w * 0.40);
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
// Back, slots, highlights
// ─────────────────────────────────────────────────────────────────

pub fn draw_card_back(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, theme: &Theme) {
    rounded_rect(cr, x + 3.0, y + 4.0, w, h, CARD_RADIUS);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.14);
    cr.fill().ok();
    rounded_rect(cr, x + 1.5, y + 2.0, w, h, CARD_RADIUS);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.22);
    cr.fill().ok();

    rounded_rect(cr, x, y, w, h, CARD_RADIUS);
    theme.card_back_color.set_source(cr);
    cr.fill_preserve().ok();
    cr.set_line_width(1.0);
    theme.card_face_border.set_source(cr);
    cr.stroke().ok();

    let inset = w * 0.07;
    rounded_rect(
        cr,
        x + inset,
        y + inset,
        w - 2.0 * inset,
        h - 2.0 * inset,
        CARD_RADIUS * 0.7,
    );
    cr.set_line_width(1.5);
    theme.card_back_pattern.set_source_alpha(cr, 0.60);
    cr.stroke().ok();

    // Inner second border (nested)
    let inset2 = w * 0.13;
    rounded_rect(
        cr,
        x + inset2,
        y + inset2,
        w - 2.0 * inset2,
        h - 2.0 * inset2,
        CARD_RADIUS * 0.5,
    );
    cr.set_line_width(0.8);
    theme.card_back_pattern.set_source_alpha(cr, 0.35);
    cr.stroke().ok();

    // Center diamond
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let dr = w * 0.16;
    cr.move_to(cx, cy - dr);
    cr.line_to(cx + dr * 0.68, cy);
    cr.line_to(cx, cy + dr);
    cr.line_to(cx - dr * 0.68, cy);
    cr.close_path();
    theme.card_back_pattern.set_source_alpha(cr, 0.55);
    cr.fill().ok();

    // Subtle diagonal cross-hatch lines for texture
    {
        let inset = w * 0.07;
        let x0 = x + inset;
        let y0 = y + inset;
        let x1 = x + w - inset;
        let y1 = y + h - inset;
        cr.save().ok();
        rounded_rect(
            cr,
            x + inset,
            y + inset,
            w - 2.0 * inset,
            h - 2.0 * inset,
            CARD_RADIUS * 0.7,
        );
        cr.clip();
        cr.set_line_width(0.5);
        theme.card_back_pattern.set_source_alpha(cr, 0.15);
        let step = w * 0.12;
        let mut xi = x0;
        while xi < x1 + (y1 - y0) {
            cr.move_to(xi, y0);
            cr.line_to(xi - (y1 - y0), y1);
            xi += step;
        }
        cr.stroke().ok();
        cr.restore().ok();
    }
}

pub fn draw_empty_slot(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, theme: &Theme) {
    rounded_rect(cr, x, y, w, h, CARD_RADIUS);
    theme.empty_slot.set_source_alpha(cr, 0.08);
    cr.fill_preserve().ok();
    cr.set_line_width(1.5);
    cr.set_dash(&[5.0, 4.0], 0.0);
    theme.empty_slot.set_source_alpha(cr, 0.42);
    cr.stroke().ok();
    cr.set_dash(&[], 0.0);
}

pub fn draw_selected_highlight(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, theme: &Theme) {
    // Glow
    rounded_rect(cr, x - 4.0, y - 4.0, w + 8.0, h + 8.0, CARD_RADIUS + 4.0);
    theme.gold.set_source_alpha(cr, 0.25);
    cr.set_line_width(6.0);
    cr.stroke().ok();

    rounded_rect(cr, x - 2.0, y - 2.0, w + 4.0, h + 4.0, CARD_RADIUS + 2.0);
    cr.set_line_width(2.5);
    theme.highlight.set_source(cr);
    cr.stroke().ok();
}

pub fn draw_hover_highlight(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, theme: &Theme) {
    rounded_rect(cr, x - 1.0, y - 1.0, w + 2.0, h + 2.0, CARD_RADIUS + 1.0);
    cr.set_line_width(2.0);
    theme.gold.set_source_alpha(cr, 0.55);
    cr.stroke().ok();
}
