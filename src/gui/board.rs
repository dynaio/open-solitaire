use gtk4::cairo;
use openpatience::core::engine::Engine;
use openpatience::core::state::{GameState, PileRef};

use super::render::card::{draw_hover_highlight, draw_selected_highlight, CARD_H, CARD_W};
use super::render::{draw_card_back, draw_card_face, draw_empty_slot, Theme};

#[derive(Clone, Copy)]
pub struct Selection {
    pub pile: PileRef,
    pub from_idx: u8,
}

pub const BOARD_MARGIN: f64 = 24.0;
pub const GAP: f64 = 16.0;
pub const TAB_GAP_DOWN: f64 = 22.0;
pub const TAB_GAP_UP: f64 = 32.0;
pub const BASE_H: f64 = BOARD_MARGIN + CARD_H + TAB_GAP_UP
    + CARD_H + 12.0 * TAB_GAP_UP + BOARD_MARGIN;

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
    w: f64, h: f64,
    engine: &Engine,
    selection: &Option<Selection>,
    hover: &Option<(PileRef, u8)>,
) {
    let theme = Theme::modern_felt();

    theme.table_bg.set_source(cr);
    cr.paint().ok();

    let (scale, off_x, off_y) = compute_layout(engine, w, h);
    let sx = |bx: f64| off_x + bx * scale;
    let sy = |by: f64| off_y + by * scale;
    let ss = |s: f64| s * scale;
    let card_w = ss(CARD_W);
    let card_h = ss(CARD_H);

    let n_free_cells = engine.state.free_cells.len();
    let n_tableau = engine.state.tableau.len();
    let bw = base_w(engine);

    // Top row layout:
    // - If no free cells (Klondike): Stock, Waste | ... | Foundation ×4
    // - If free cells (FreeCell): FreeCell ×4 | ... | Foundation ×4
    if n_free_cells == 0 {
        // Klondike-style top row
        let stock_x = sx(BOARD_MARGIN);
        let stock_y = sy(BOARD_MARGIN);
        if engine.state.stock.is_empty() {
            draw_empty_slot(cr, stock_x, stock_y, card_w, card_h, &theme);
        } else {
            draw_card_back(cr, stock_x, stock_y, card_w, card_h, &theme);
        }

        let waste_x = sx(BOARD_MARGIN + CARD_W + GAP);
        if let Some(card) = engine.state.waste.last() {
            draw_card_face(cr, waste_x, stock_y, card_w, card_h, *card, &theme);
            if let Some(sel) = selection {
                if sel.pile == PileRef::Waste {
                    draw_selected_highlight(cr, waste_x, stock_y, card_w, card_h, &theme);
                }
            }
        } else {
            draw_empty_slot(cr, waste_x, stock_y, card_w, card_h, &theme);
        }
    } else {
        // FreeCell-style: 4 free cells on the left
        for i in 0..n_free_cells {
            let fx = sx(BOARD_MARGIN + (i as f64) * (CARD_W + GAP));
            let fy = sy(BOARD_MARGIN);
            match engine.state.free_cells[i] {
                Some(card) => {
                    draw_card_face(cr, fx, fy, card_w, card_h, card, &theme);
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

    // Foundations (top-right, always 4)
    let n_foundation = engine.state.foundation.len();
    let foundation_total_w =
        n_foundation as f64 * CARD_W + (n_foundation as f64 - 1.0) * GAP;
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
        }
    }

    // Tableau
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
            if placed.face_up {
                draw_card_face(cr, tx, y, card_w, card_h, placed.card, &theme);
            } else {
                draw_card_back(cr, tx, y, card_w, card_h, &theme);
            }
            // Hover
            if let Some(hv) = hover {
                if hv.0 == PileRef::Tableau(t as u8) && hv.1 == i as u8 && placed.face_up {
                    draw_hover_highlight(cr, tx, y, card_w, card_h, &theme);
                }
            }
            // Selection
            if let Some(sel) = selection {
                if sel.pile == PileRef::Tableau(t as u8) && sel.from_idx == i as u8 {
                    draw_selected_highlight(cr, tx, y, card_w, card_h, &theme);
                }
            }
            if i + 1 < pile.len() {
                let gap = if placed.face_up { TAB_GAP_UP } else { TAB_GAP_DOWN };
                y += ss(gap);
            }
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
        // Klondike
        if bx >= BOARD_MARGIN && bx <= BOARD_MARGIN + CARD_W
            && by >= BOARD_MARGIN && by <= BOARD_MARGIN + CARD_H
        {
            return Some((PileRef::Stock, 0));
        }
        let waste_x = BOARD_MARGIN + CARD_W + GAP;
        if bx >= waste_x && bx <= waste_x + CARD_W
            && by >= BOARD_MARGIN && by <= BOARD_MARGIN + CARD_H
        {
            let idx = state.waste.len().saturating_sub(1) as u8;
            return Some((PileRef::Waste, idx));
        }
    } else {
        // FreeCell
        for i in 0..n_free_cells {
            let fx = BOARD_MARGIN + (i as f64) * (CARD_W + GAP);
            if bx >= fx && bx <= fx + CARD_W
                && by >= BOARD_MARGIN && by <= BOARD_MARGIN + CARD_H
            {
                return Some((PileRef::FreeCell(i as u8), 0));
            }
        }
    }

    // Foundation
    let n_foundation = state.foundation.len();
    let foundation_total_w =
        n_foundation as f64 * CARD_W + (n_foundation as f64 - 1.0) * GAP;
    let foundation_x_start = base_w - BOARD_MARGIN - foundation_total_w;
    for i in 0..n_foundation {
        let fx = foundation_x_start + (i as f64) * (CARD_W + GAP);
        if bx >= fx && bx <= fx + CARD_W
            && by >= BOARD_MARGIN && by <= BOARD_MARGIN + CARD_H
        {
            let idx = state.foundation[i].len().saturating_sub(1) as u8;
            return Some((PileRef::Foundation(i as u8), idx));
        }
    }

    // Tableau
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
                let gap = if placed.face_up { TAB_GAP_UP } else { TAB_GAP_DOWN };
                y += gap;
            }
        }
        if let Some(i) = hit {
            return Some((PileRef::Tableau(t as u8), i as u8));
        }
    }

    None
}
