use std::cell::Cell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk::glib;
use libadwaita as adw;
use gtk::prelude::*;

use openpatience::core::engine::Engine;
use openpatience::core::moves::Move;
use openpatience::core::rules::RuleSet;
use openpatience::core::solver::find_solvable_seed;
use openpatience::core::state::PileRef;

use super::board::{base_w, compute_layout, draw_board, hit_test, DragState, Selection};
use super::settings::{apply_difficulty, show_settings_dialog, Difficulty};

const APP_ID: &str = "io.github.abdelkader.OpenSolitaire";

const MODES: &[(&str, &str)] = &[
    ("Klondike",    include_str!("../../assets/rules/klondike.json")),
    ("Klondike ×3", include_str!("../../assets/rules/klondike-draw3.json")),
    ("FreeCell",    include_str!("../../assets/rules/freecell.json")),
];

fn new_engine(mode_idx: usize, difficulty: Difficulty) -> Engine {
    let mut rule = RuleSet::from_json(MODES[mode_idx].1).expect("rule");
    if mode_idx < 2 {
        apply_difficulty(&mut rule, difficulty);
    }
    let seed = find_solvable_seed(&rule, 1, 200);
    Engine::new(rule, seed)
}

pub fn run() -> glib::ExitCode {
    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &gtk::Application) {
    let difficulty = Rc::new(Cell::new(Difficulty::Normal));
    let mode_idx = Rc::new(Cell::new(0usize));

    let engine: Rc<std::cell::RefCell<Engine>> =
        Rc::new(std::cell::RefCell::new(new_engine(0, Difficulty::Normal)));
    let selection: Rc<std::cell::RefCell<Option<Selection>>> = Rc::new(std::cell::RefCell::new(None));
    let hover: Rc<std::cell::RefCell<Option<(PileRef, u8)>>> = Rc::new(std::cell::RefCell::new(None));
    let drag: Rc<std::cell::RefCell<Option<DragState>>> = Rc::new(std::cell::RefCell::new(None));
    let dims = Rc::new(Cell::new((1000.0f64, 800.0f64)));

    let area = gtk::DrawingArea::new();
    area.set_hexpand(true);
    area.set_vexpand(true);

    area.set_draw_func({
        let engine = engine.clone();
        let selection = selection.clone();
        let hover = hover.clone();
        let drag = drag.clone();
        let dims = dims.clone();
        move |_area, cr, w, h| {
            dims.set((w as f64, h as f64));
            draw_board(
                cr, w as f64, h as f64,
                &engine.borrow(),
                &selection.borrow(),
                &hover.borrow(),
                &drag.borrow(),
            );
        }
    });

    // Hover
    let motion = gtk::EventControllerMotion::new();
    motion.connect_motion({
        let engine = engine.clone();
        let hover = hover.clone();
        let dims = dims.clone();
        let area = area.clone();
        move |_m, x, y| {
            let (w, h) = dims.get();
            let (scale, off_x, off_y) = compute_layout(&engine.borrow(), w, h);
            let bw = base_w(&engine.borrow());
            let hit = { let e = engine.borrow(); hit_test(&e.state, bw, scale, off_x, off_y, x, y) };
            if *hover.borrow() != hit {
                *hover.borrow_mut() = hit;
                area.queue_draw();
            }
        }
    });
    area.add_controller(motion);

    // -------- Drag gesture (also handles click) --------
    let drag_gesture = gtk::GestureDrag::new();
    drag_gesture.set_button(1);

    drag_gesture.connect_drag_begin({
        let engine = engine.clone();
        let drag = drag.clone();
        let dims = dims.clone();
        let area = area.clone();
        move |_g, start_x, start_y| {
            let (w, h) = dims.get();
            let (scale, off_x, off_y) = compute_layout(&engine.borrow(), w, h);
            let bw = base_w(&engine.borrow());
            let hit = { let e = engine.borrow(); hit_test(&e.state, bw, scale, off_x, off_y, start_x, start_y) };
            if let Some((pile, idx)) = hit {
                // Stock click: draw immediately, no drag
                if let PileRef::Stock = pile {
                    let mut e = engine.borrow_mut();
                    if !e.state.stock.is_empty() { e.apply(Move::Draw); }
                    else { e.apply(Move::Recycle); }
                    drop(e);
                    area.queue_draw();
                    return;
                }
                // Only start drag from movable face-up sources
                let e = engine.borrow();
                let cards: Vec<_> = match pile {
                    PileRef::Tableau(t) => {
                        let p = &e.state.tableau[t as usize];
                        if idx as usize >= p.len() || !p[idx as usize].face_up { return; }
                        p[idx as usize..].iter().map(|pc| pc.card).collect()
                    }
                    PileRef::Waste => {
                        match e.state.waste.last() { Some(c) => vec![*c], None => return }
                    }
                    PileRef::FreeCell(f) => {
                        match e.state.free_cells[f as usize] { Some(c) => vec![c], None => return }
                    }
                    _ => return,
                };
                drop(e);
                *drag.borrow_mut() = Some(DragState {
                    pile, from_idx: idx, cards,
                    start_x, start_y,
                    mouse_x: start_x, mouse_y: start_y,
                });
                area.queue_draw();
            }
        }
    });

    drag_gesture.connect_drag_update({
        let drag = drag.clone();
        let area = area.clone();
        move |_g, offset_x, offset_y| {
            if let Some(d) = drag.borrow_mut().as_mut() {
                d.mouse_x = d.start_x + offset_x;
                d.mouse_y = d.start_y + offset_y;
                area.queue_draw();
            }
        }
    });

    drag_gesture.connect_drag_end({
        let engine = engine.clone();
        let drag = drag.clone();
        let selection = selection.clone();
        let dims = dims.clone();
        let area = area.clone();
        move |_g, offset_x, offset_y| {
            let (w, h) = dims.get();
            let (scale, off_x, off_y) = compute_layout(&engine.borrow(), w, h);
            let bw = base_w(&engine.borrow());
            let end_x = {
                let d = drag.borrow();
                d.as_ref().map(|x| x.start_x + offset_x).unwrap_or(offset_x)
            };
            let end_y = {
                let d = drag.borrow();
                d.as_ref().map(|x| x.start_y + offset_y).unwrap_or(offset_y)
            };
            let dragged = drag.borrow_mut().take();
            let target = hit_test(&engine.borrow().state, bw, scale, off_x, off_y, end_x, end_y);

            let distance = (offset_x.powi(2) + offset_y.powi(2)).sqrt();

            if let Some(d) = dragged {
                if distance > 8.0 {
                    if let Some((to_pile, _)) = target {
                        let m = Move::Move {
                            from: d.pile, from_idx: d.from_idx, to: to_pile,
                        };
                        engine.borrow_mut().apply(m);
                        *selection.borrow_mut() = None;
                    }
                } else {
                    // Short drag = click: toggle selection
                    let cur = Selection { pile: d.pile, from_idx: d.from_idx };
                    let mut sel = selection.borrow_mut();
                    *sel = match *sel {
                        Some(s) if s.pile == cur.pile && s.from_idx == cur.from_idx => None,
                        _ => Some(cur),
                    };
                }
            } else if distance <= 8.0 {
                // Click without starting a drag (e.g., empty slot) — try select/deselect
                if let Some((pile, idx)) = target {
                    let cur = Selection { pile, from_idx: idx };
                    let current = *selection.borrow();
                    let next = match current {
                        Some(s) if s.pile == cur.pile && s.from_idx == cur.from_idx => None,
                        None => Some(cur),
                        Some(s) => {
                            let m = Move::Move { from: s.pile, from_idx: s.from_idx, to: pile };
                            if engine.borrow_mut().apply(m) { None } else { Some(cur) }
                        }
                    };
                    *selection.borrow_mut() = next;
                } else {
                    *selection.borrow_mut() = None;
                }
            }
            area.queue_draw();
        }
    });
    area.add_controller(drag_gesture);

    // Keyboard
    let key = gtk::EventControllerKey::new();
    key.connect_key_pressed({
        let engine = engine.clone();
        let area = area.clone();
        move |_ctrl, keyval, _keycode, _state| {
            let name = keyval.name().map(|s| s.to_string()).unwrap_or_default();
            let mut e = engine.borrow_mut();
            let handled = match name.as_str() {
                "u" | "U" => { e.undo(); true }
                "h" | "H" => {
                    if let Some(m) = e.hint() { e.apply(m); }
                    true
                }
                "f" | "F" => { e.auto_foundation_all(); true }
                "d" | "D" => { e.apply(Move::Draw); true }
                _ => false,
            };
            drop(e);
            if handled { area.queue_draw(); glib::Propagation::Stop }
            else { glib::Propagation::Proceed }
        }
    });
    area.add_controller(key);

    // Header
    let header = adw::HeaderBar::new();

    let undo_btn = gtk::Button::builder()
        .icon_name("edit-undo-symbolic").tooltip_text("Undo (U)").build();
    undo_btn.connect_clicked({
        let engine = engine.clone(); let area = area.clone();
        move |_| { engine.borrow_mut().undo(); area.queue_draw(); }
    });
    header.pack_start(&undo_btn);

    let new_btn = gtk::Button::builder()
        .icon_name("document-new-symbolic").tooltip_text("New game").build();
    new_btn.connect_clicked({
        let engine = engine.clone();
        let mode_idx = mode_idx.clone();
        let difficulty = difficulty.clone();
        let area = area.clone();
        move |_| {
            *engine.borrow_mut() = new_engine(mode_idx.get(), difficulty.get());
            area.queue_draw();
        }
    });
    header.pack_start(&new_btn);

    let hint_btn = gtk::Button::builder()
        .icon_name("dialog-question-symbolic").tooltip_text("Hint (H)").build();
    hint_btn.connect_clicked({
        let engine = engine.clone(); let area = area.clone();
        move |_| {
            let mut e = engine.borrow_mut();
            if let Some(m) = e.hint() { e.apply(m); }
            drop(e);
            area.queue_draw();
        }
    });
    header.pack_start(&hint_btn);

    let mode_strings: Vec<&str> = MODES.iter().map(|(n, _)| *n).collect();
    let mode_drop = gtk::DropDown::from_strings(&mode_strings);
    mode_drop.set_selected(0);
    mode_drop.set_tooltip_text(Some("Game mode"));
    mode_drop.connect_selected_notify({
        let engine = engine.clone();
        let mode_idx = mode_idx.clone();
        let difficulty = difficulty.clone();
        let area = area.clone();
        move |dd| {
            let idx = dd.selected() as usize;
            if idx >= MODES.len() { return; }
            mode_idx.set(idx);
            *engine.borrow_mut() = new_engine(idx, difficulty.get());
            area.queue_draw();
        }
    });
    header.pack_start(&mode_drop);

    let auto_btn = gtk::Button::builder()
        .icon_name("go-up-symbolic").tooltip_text("Auto-foundation (F)").build();
    auto_btn.connect_clicked({
        let engine = engine.clone(); let area = area.clone();
        move |_| { engine.borrow_mut().auto_foundation_all(); area.queue_draw(); }
    });
    header.pack_end(&auto_btn);

    let settings_btn = gtk::Button::builder()
        .icon_name("emblem-system-symbolic").tooltip_text("Settings").build();
    header.pack_end(&settings_btn);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Open Solitaire")
        .default_width(1000)
        .default_height(800)
        .build();

    // Settings button: needs a window reference
    {
        let engine = engine.clone();
        let difficulty = difficulty.clone();
        let area = area.clone();
        let mode_idx = mode_idx.clone();
        let win = window.clone();
        settings_btn.connect_clicked(move |_| {
            let rule = engine.borrow().rules.clone();
            let diff = difficulty.clone();
            let eng = engine.clone();
            let area2 = area.clone();
            let mi = mode_idx.clone();
            show_settings_dialog(&win, &rule, diff, move |new_diff| {
                // Apply difficulty + restart with solvable deal
                let _ = &mi;
                *eng.borrow_mut() = new_engine(mi.get(), new_diff);
                area2.queue_draw();
            });
        });
    }

    window.set_titlebar(Some(&header));
    window.set_child(Some(&area));
    window.present();
}
