use gtk4::cairo;
use openpatience::core::card::Card;
use openpatience::core::engine::Engine;
use openpatience::core::state::{GameState, PileRef};
use std::time::Instant;

use super::render::card::{
    draw_hover_highlight, draw_selected_highlight, draw_suit_shape, CARD_H, CARD_W,
};
use super::render::{draw_card_back, draw_card_face, draw_empty_slot, Theme};
use openpatience::core::card::Suit;

#[derive(Clone, Copy)]
pub struct Selection {
    pub pile: PileRef,
    pub from_idx: u8,
}

pub struct VictoryAnimation {
    cards: Vec<Card>,
    started: Instant,
}

impl VictoryAnimation {
    pub fn new(state: &GameState) -> Self {
        let cards = state.foundation.iter().flatten().copied().collect();

        Self {
            cards,
            started: Instant::now(),
        }
    }

    pub fn is_finished(&self) -> bool {
        let total = 2.6 + self.cards.len() as f64 * 0.025;
        self.started.elapsed().as_secs_f64() >= total
    }
}

/// Draws the completed game's cards as a staggered, cascading victory animation.
pub fn draw_victory_animation(cr: &cairo::Context, w: f64, h: f64, animation: &VictoryAnimation) {
    let elapsed = animation.started.elapsed().as_secs_f64();
    let card_w = (CARD_W * 0.78).min(w * 0.16);
    let card_h = card_w * CARD_H / CARD_W;
    let theme = Theme::deep_emerald();
    let flight_duration = 2.6;

    for (index, card) in animation.cards.iter().enumerate() {
        let delay = index as f64 * 0.025;
        let local_time = elapsed - delay;

        if local_time < 0.0 || local_time > flight_duration {
            continue;
        }

        let t = (local_time / flight_duration).clamp(0.0, 1.0);
        let eased = t * t * (3.0 - 2.0 * t);

        // Cards launch from the foundation row.
        let start_x = w * (0.20 + card.suit.as_u8() as f64 * 0.20);
        let start_y = 28.0;

        // Deterministic spread across the full board.
        let spread = ((index * 37) % 997) as f64 / 997.0;
        let end_x = 12.0 + spread * (w - card_w - 24.0);
        let end_y = h + card_h;

        let x = start_x + (end_x - start_x) * eased;
        let arc = (std::f64::consts::PI * t).sin() * h * 0.48;
        let y = start_y + (end_y - start_y) * eased - arc;

        let direction = if index % 2 == 0 { 1.0 } else { -1.0 };
        let rotation = direction * 0.85 * eased;

        cr.save().ok();
        cr.translate(x + card_w / 2.0, y + card_h / 2.0);
        cr.rotate(rotation);

        draw_card_face(
            cr,
            -card_w / 2.0,
            -card_h / 2.0,
            card_w,
            card_h,
            *card,
            &theme,
        );

        cr.restore().ok();
    }
}

pub struct DragState {
    pub pile: PileRef,
    pub from_idx: u8,
    pub cards: Vec<Card>,
    pub start_x: f64,
    pub start_y: f64,
    pub mouse_x: f64,
    pub mouse_y: f64,
}

pub const BOARD_MARGIN: f64 = 24.0;
pub const GAP: f64 = 16.0;
pub const TAB_GAP_DOWN: f64 = 22.0;
pub const TAB_GAP_UP: f64 = 32.0;
pub const BASE_H: f64 =
    BOARD_MARGIN + CARD_H + TAB_GAP_UP + CARD_H + 12.0 * TAB_GAP_UP + BOARD_MARGIN;

pub fn base_w(engine: &Engine) -> f64 {
    let piles = engine.state.tableau.len() as f64;
    2.0 * BOARD_MARGIN + piles * CARD_W + (piles - 1.0) * GAP
}

pub fn compute_layout(engine: &Engine, w: f64, h: f64) -> (f64, f64, f64) {
    let bw = base_w(engine);
    let scale = (w / bw).min(h / BASE_H).clamp(0.35, 2.5);
    let off_x = (w - bw * scale) / 2.0;
    let off_y = (h - BASE_H * scale) / 2.0;
    (scale, off_x, off_y)
}

pub fn draw_board(
    cr: &cairo::Context,
    w: f64,
    h: f64,
    engine: &Engine,
    selection: &Option<Selection>,
    hover: &Option<(PileRef, u8)>,
    drag: &Option<DragState>,
) {
    let theme = Theme::deep_emerald();
    // Radial gradient: warm emerald center, darker edges
    let cx = w / 2.0;
    let cy = h / 2.0;
    let r = (w.max(h)) * 0.7;
    let grad = cairo::RadialGradient::new(cx, cy, 0.0, cx, cy, r);
    grad.add_color_stop_rgb(
        0.0,
        theme.table_bg_center.0,
        theme.table_bg_center.1,
        theme.table_bg_center.2,
    );
    grad.add_color_stop_rgb(0.7, theme.table_bg.0, theme.table_bg.1, theme.table_bg.2);
    grad.add_color_stop_rgb(
        1.0,
        theme.table_bg.0 * 0.70,
        theme.table_bg.1 * 0.70,
        theme.table_bg.2 * 0.70,
    );
    cr.set_source(&grad).ok();
    cr.paint().ok();

    // Score banner (subtle, top center)
    let score_text = format!("Score: {}", engine.state.score);
    cr.select_font_face(
        "Noto Sans",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
    );
    cr.set_font_size(16.0);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.35);
    if let Ok(ext) = cr.text_extents(&score_text) {
        cr.move_to(w / 2.0 - ext.width() / 2.0, 22.0);
        cr.show_text(&score_text).ok();
    }

    let (scale, off_x, off_y) = compute_layout(engine, w, h);
    let sx = |bx: f64| off_x + bx * scale;
    let sy = |by: f64| off_y + by * scale;
    let ss = |s: f64| s * scale;
    let card_w = ss(CARD_W);
    let card_h = ss(CARD_H);

    let n_free_cells = engine.state.free_cells.len();
    let n_tableau = engine.state.tableau.len();
    let bw = base_w(engine);

    // --- Top row ---
    if n_free_cells == 0 {
        let stock_x = sx(BOARD_MARGIN);
        let stock_y = sy(BOARD_MARGIN);
        if engine.state.stock.is_empty() {
            draw_empty_slot(cr, stock_x, stock_y, card_w, card_h, &theme);
        } else {
            draw_card_back(cr, stock_x, stock_y, card_w, card_h, &theme);
        }
        let waste_x = sx(BOARD_MARGIN + CARD_W + GAP);
        if let Some(card) = engine.state.waste.last() {
            let is_dragged = drag
                .as_ref()
                .map(|d| d.pile == PileRef::Waste)
                .unwrap_or(false);
            if !is_dragged {
                draw_card_face(cr, waste_x, stock_y, card_w, card_h, *card, &theme);
            }
            if let Some(sel) = selection {
                if sel.pile == PileRef::Waste {
                    draw_selected_highlight(cr, waste_x, stock_y, card_w, card_h, &theme);
                }
            }
        } else {
            draw_empty_slot(cr, waste_x, stock_y, card_w, card_h, &theme);
        }
    } else {
        for i in 0..n_free_cells {
            let fx = sx(BOARD_MARGIN + (i as f64) * (CARD_W + GAP));
            let fy = sy(BOARD_MARGIN);
            let is_dragged = drag
                .as_ref()
                .map(|d| d.pile == PileRef::FreeCell(i as u8))
                .unwrap_or(false);
            match engine.state.free_cells[i] {
                Some(card) => {
                    if !is_dragged {
                        draw_card_face(cr, fx, fy, card_w, card_h, card, &theme);
                    }
                    if let Some(sel) = selection {
                        if sel.pile == PileRef::FreeCell(i as u8) {
                            draw_selected_highlight(cr, fx, fy, card_w, card_h, &theme);
                        }
                    }
                }
                None => draw_empty_slot(cr, fx, fy, card_w, card_h, &theme),
            }
        }
    }

    // Foundations (top-right)
    let n_foundation = engine.state.foundation.len();
    let foundation_total_w = n_foundation as f64 * CARD_W + (n_foundation as f64 - 1.0) * GAP;
    let foundation_x_start = bw - BOARD_MARGIN - foundation_total_w;
    for i in 0..n_foundation {
        let fx = sx(foundation_x_start + (i as f64) * (CARD_W + GAP));
        let fy = sy(BOARD_MARGIN);
        if let Some(card) = engine.state.foundation[i].last() {
            draw_card_face(cr, fx, fy, card_w, card_h, *card, &theme);
            if let Some(sel) = selection {
                if sel.pile == PileRef::Foundation(i as u8) {
                    draw_selected_highlight(cr, fx, fy, card_w, card_h, &theme);
                }
            }
        } else {
            draw_empty_slot(cr, fx, fy, card_w, card_h, &theme);
            // Suit hint in empty foundation slot
            let suit = match i % 4 {
                0 => Suit::Spades,
                1 => Suit::Hearts,
                2 => Suit::Diamonds,
                _ => Suit::Clubs,
            };
            let suit_color = theme.suit_color(suit);
            suit_color.set_source_alpha(cr, 0.30);
            draw_suit_shape(
                cr,
                suit,
                fx + card_w / 2.0,
                fy + card_h / 2.0,
                card_w * 0.22,
            );
        }
    }

    // --- Tableau (skip cards currently being dragged) ---
    let tableau_y = sy(BOARD_MARGIN + CARD_H + TAB_GAP_UP);
    for t in 0..n_tableau {
        let tx = sx(BOARD_MARGIN + (t as f64) * (CARD_W + GAP));
        let pile = &engine.state.tableau[t];
        if pile.is_empty() {
            draw_empty_slot(cr, tx, tableau_y, card_w, card_h, &theme);
            continue;
        }
        let mut y = tableau_y;
        for (i, placed) in pile.iter().enumerate() {
            let is_dragged = drag
                .as_ref()
                .map(|d| d.pile == PileRef::Tableau(t as u8) && i >= d.from_idx as usize)
                .unwrap_or(false);
            if !is_dragged {
                if placed.face_up {
                    draw_card_face(cr, tx, y, card_w, card_h, placed.card, &theme);
                } else {
                    draw_card_back(cr, tx, y, card_w, card_h, &theme);
                }
            }
            if let Some(hv) = hover {
                if hv.0 == PileRef::Tableau(t as u8)
                    && hv.1 == i as u8
                    && placed.face_up
                    && !is_dragged
                {
                    draw_hover_highlight(cr, tx, y, card_w, card_h, &theme);
                }
            }
            if let Some(sel) = selection {
                if sel.pile == PileRef::Tableau(t as u8) && sel.from_idx == i as u8 {
                    draw_selected_highlight(cr, tx, y, card_w, card_h, &theme);
                }
            }
            if i + 1 < pile.len() {
                y += ss(if placed.face_up {
                    TAB_GAP_UP
                } else {
                    TAB_GAP_DOWN
                });
            }
        }
    }

    // --- Dragged cards float on top, following the mouse ---
    if let Some(d) = drag {
        let (orig_x, orig_y) = match d.pile {
            PileRef::Tableau(t) => {
                let tx = sx(BOARD_MARGIN + (t as f64) * (CARD_W + GAP));
                let pile = &engine.state.tableau[t as usize];
                let mut y = sy(BOARD_MARGIN + CARD_H + TAB_GAP_UP);
                for (i, p) in pile.iter().enumerate() {
                    if i >= d.from_idx as usize {
                        break;
                    }
                    y += ss(if p.face_up { TAB_GAP_UP } else { TAB_GAP_DOWN });
                }
                (tx, y)
            }
            PileRef::Waste => (sx(BOARD_MARGIN + CARD_W + GAP), sy(BOARD_MARGIN)),
            PileRef::FreeCell(f) => (
                sx(BOARD_MARGIN + (f as f64) * (CARD_W + GAP)),
                sy(BOARD_MARGIN),
            ),
            _ => (0.0, 0.0),
        };
        let dx = d.mouse_x - d.start_x;
        let dy = d.mouse_y - d.start_y;
        let mut cy = orig_y + dy;
        for c in &d.cards {
            draw_card_face(cr, orig_x + dx, cy, card_w, card_h, *c, &theme);
            cy += ss(TAB_GAP_UP);
        }
    }
}

pub fn hit_test(
    state: &GameState,
    base_w: f64,
    scale: f64,
    off_x: f64,
    off_y: f64,
    mx: f64,
    my: f64,
) -> Option<(PileRef, u8)> {
    let bx = (mx - off_x) / scale;
    let by = (my - off_y) / scale;
    let n_free_cells = state.free_cells.len();

    if n_free_cells == 0 {
        if bx >= BOARD_MARGIN
            && bx <= BOARD_MARGIN + CARD_W
            && by >= BOARD_MARGIN
            && by <= BOARD_MARGIN + CARD_H
        {
            return Some((PileRef::Stock, 0));
        }
        let waste_x = BOARD_MARGIN + CARD_W + GAP;
        if bx >= waste_x
            && bx <= waste_x + CARD_W
            && by >= BOARD_MARGIN
            && by <= BOARD_MARGIN + CARD_H
        {
            return Some((PileRef::Waste, state.waste.len().saturating_sub(1) as u8));
        }
    } else {
        for i in 0..n_free_cells {
            let fx = BOARD_MARGIN + (i as f64) * (CARD_W + GAP);
            if bx >= fx && bx <= fx + CARD_W && by >= BOARD_MARGIN && by <= BOARD_MARGIN + CARD_H {
                return Some((PileRef::FreeCell(i as u8), 0));
            }
        }
    }

    let n_foundation = state.foundation.len();
    let foundation_total_w = n_foundation as f64 * CARD_W + (n_foundation as f64 - 1.0) * GAP;
    let foundation_x_start = base_w - BOARD_MARGIN - foundation_total_w;
    for i in 0..n_foundation {
        let fx = foundation_x_start + (i as f64) * (CARD_W + GAP);
        if bx >= fx && bx <= fx + CARD_W && by >= BOARD_MARGIN && by <= BOARD_MARGIN + CARD_H {
            return Some((
                PileRef::Foundation(i as u8),
                state.foundation[i].len().saturating_sub(1) as u8,
            ));
        }
    }

    let tableau_y = BOARD_MARGIN + CARD_H + TAB_GAP_UP;
    for t in 0..state.tableau.len() {
        let tx = BOARD_MARGIN + (t as f64) * (CARD_W + GAP);
        if bx < tx || bx > tx + CARD_W {
            continue;
        }
        let pile = &state.tableau[t];
        if pile.is_empty() {
            if by >= tableau_y && by <= tableau_y + CARD_H {
                return Some((PileRef::Tableau(t as u8), 0));
            }
            continue;
        }
        let mut y = tableau_y;
        let mut hit: Option<usize> = None;
        for (i, placed) in pile.iter().enumerate() {
            if by >= y && by < y + CARD_H {
                hit = Some(i);
            }
            if i + 1 < pile.len() {
                y += if placed.face_up {
                    TAB_GAP_UP
                } else {
                    TAB_GAP_DOWN
                };
            }
        }
        if let Some(i) = hit {
            return Some((PileRef::Tableau(t as u8), i as u8));
        }
    }
    None
}
