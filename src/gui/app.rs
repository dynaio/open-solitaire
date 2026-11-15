use std::cell::Cell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk::glib;
use libadwaita as adw;
use gtk::prelude::*;

use openpatience::core::engine::Engine;
use openpatience::core::moves::Move;
use openpatience::core::rules::RuleSet;
use openpatience::core::state::PileRef;

use super::board::{base_w, compute_layout, draw_board, hit_test, Selection};

const APP_ID: &str = "io.github.abdelkader.OpenSolitaire";

const MODES: &[(&str, &str)] = &[
    ("Klondike",      include_str!("../../assets/rules/klondike.json")),
    ("Klondike ×3",   include_str!("../../assets/rules/klondike-draw3.json")),
    ("FreeCell",      include_str!("../../assets/rules/freecell.json")),
];

pub fn run() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &gtk::Application) {
    let initial_rule = RuleSet::from_json(MODES[0].1).expect("klondike rule");

    let engine: Rc<std::cell::RefCell<Engine>> =
        Rc::new(std::cell::RefCell::new(Engine::new(initial_rule, 12345)));
    let selection: Rc<std::cell::RefCell<Option<Selection>>> =
        Rc::new(std::cell::RefCell::new(None));
    let hover: Rc<std::cell::RefCell<Option<(PileRef, u8)>>> =
        Rc::new(std::cell::RefCell::new(None));
    let dims = Rc::new(Cell::new((900.0_f64, 750.0_f64)));

    let area = gtk::DrawingArea::new();
    area.set_hexpand(true);
    area.set_vexpand(true);

    area.set_draw_func({
        let engine = engine.clone();
        let selection = selection.clone();
        let hover = hover.clone();
        let dims = dims.clone();
        move |_area, cr, w, h| {
            dims.set((w as f64, h as f64));
            draw_board(
                cr, w as f64, h as f64,
                &engine.borrow(),
                &selection.borrow(),
                &hover.borrow(),
            );
        }
    });

    // --- Mouse: motion for hover ---
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
            let hit = {
                let e = engine.borrow();
                hit_test(&e.state, bw, scale, off_x, off_y, x, y)
            };
            let old = *hover.borrow();
            if old != hit {
                *hover.borrow_mut() = hit;
                area.queue_draw();
            }
        }
    });
    area.add_controller(motion);

    // --- Mouse: click ---
    let click = gtk::GestureClick::new();
    click.connect_pressed({
        let engine = engine.clone();
        let selection = selection.clone();
        let dims = dims.clone();
        let area = area.clone();
        move |_gesture, _n, x, y| {
            let (w, h) = dims.get();
            let (scale, off_x, off_y) = compute_layout(&engine.borrow(), w, h);
            let bw = base_w(&engine.borrow());
            let hit = {
                let e = engine.borrow();
                hit_test(&e.state, bw, scale, off_x, off_y, x, y)
            };
            {
                let mut e = engine.borrow_mut();
                let mut sel = selection.borrow_mut();
                match hit {
                    Some((PileRef::Stock, _)) => {
                        if !e.state.stock.is_empty() {
                            e.apply(Move::Draw);
                        } else {
                            e.apply(Move::Recycle);
                        }
                        *sel = None;
                    }
                    Some((pile, idx)) => match *sel {
                        None => {
                            *sel = Some(Selection { pile, from_idx: idx });
                        }
                        Some(s) => {
                            if s.pile == pile {
                                *sel = None;
                            } else {
                                let m = Move::Move {
                                    from: s.pile,
                                    from_idx: s.from_idx,
                                    to: pile,
                                };
                                if e.apply(m) {
                                    *sel = None;
                                } else {
                                    *sel = Some(Selection { pile, from_idx: idx });
                                }
                            }
                        }
                    },
                    None => {
                        *sel = None;
                    }
                }
            }
            area.queue_draw();
        }
    });
    area.add_controller(click);

    // --- Keyboard ---
    let key = gtk::EventControllerKey::new();
    key.connect_key_pressed({
        let engine = engine.clone();
        let area = area.clone();
        move |_ctrl, keyval, _keycode, _state| {
            let name = keyval.name().map(|s| s.to_string()).unwrap_or_default();
            let mut e = engine.borrow_mut();
            let handled = match name.as_str() {
                "u" | "U" => { e.undo(); true }
                "n" | "N" => {
                    let rules = e.rules.clone();
                    *e = Engine::new(rules, 12345);
                    true
                }
                "d" | "D" => { e.apply(Move::Draw); true }
                "f" | "F" => {
                    for i in 0..e.state.tableau.len() {
                        if e.state.pile_len(PileRef::Tableau(i as u8)) > 0 {
                            let _ = e.apply(Move::AutoFoundation(PileRef::Tableau(i as u8)));
                        }
                    }
                    true
                }
                _ => false,
            };
            drop(e);
            if handled {
                area.queue_draw();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    });
    area.add_controller(key);

    // --- Header ---
    let header = adw::HeaderBar::new();

    let undo_btn = gtk::Button::builder()
        .icon_name("edit-undo-symbolic")
        .tooltip_text("Undo (U)")
        .build();
    undo_btn.connect_clicked({
        let engine = engine.clone();
        let area = area.clone();
        move |_| {
            engine.borrow_mut().undo();
            area.queue_draw();
        }
    });
    header.pack_start(&undo_btn);

    let new_btn = gtk::Button::builder()
        .icon_name("document-new-symbolic")
        .tooltip_text("New game (N)")
        .build();
    new_btn.connect_clicked({
        let engine = engine.clone();
        let area = area.clone();
        move |_| {
            let rules = engine.borrow().rules.clone();
            *engine.borrow_mut() = Engine::new(rules, 12345);
            area.queue_draw();
        }
    });
    header.pack_start(&new_btn);

    // Mode picker (dropdown)
    let mode_strings: Vec<&str> = MODES.iter().map(|(n, _)| *n).collect();
    let mode_drop = gtk::DropDown::from_strings(&mode_strings);
    mode_drop.set_selected(0);
    mode_drop.set_tooltip_text(Some("Game mode"));
    mode_drop.connect_selected_notify({
        let engine = engine.clone();
        let area = area.clone();
        move |dd| {
            let idx = dd.selected() as usize;
            if idx >= MODES.len() { return; }
            let rule = RuleSet::from_json(MODES[idx].1).expect("rule");
            *engine.borrow_mut() = Engine::new(rule, 12345);
            area.queue_draw();
        }
    });
    header.pack_start(&mode_drop);

    let auto_btn = gtk::Button::builder()
        .icon_name("go-up-symbolic")
        .tooltip_text("Auto-move to foundation (F)")
        .build();
    auto_btn.connect_clicked({
        let engine = engine.clone();
        let area = area.clone();
        move |_| {
            let mut e = engine.borrow_mut();
            for i in 0..e.state.tableau.len() {
                if e.state.pile_len(PileRef::Tableau(i as u8)) > 0 {
                    let _ = e.apply(Move::AutoFoundation(PileRef::Tableau(i as u8)));
                }
            }
            drop(e);
            area.queue_draw();
        }
    });
    header.pack_end(&auto_btn);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Open Solitaire")
        .default_width(1000)
        .default_height(800)
        .build();
    window.set_titlebar(Some(&header));
    window.set_child(Some(&area));
    window.present();
}
