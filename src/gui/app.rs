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
    // Instant startup: seed 1, no solving yet. `ensure_solution` is called on
    // first hint click (cached afterwards), and also warmed up in background
    // by `spawn_background_solve` right after the window is shown.
    Engine::new(rule, 1)
}

/// Solve seed 1 in a background thread; attach the result to the Engine when done.
/// The window is fully interactive while this runs — user won't notice.
fn spawn_background_solve(engine: Rc<std::cell::RefCell<Engine>>) {
    use gtk::glib;
    use std::sync::Mutex;
    use std::sync::mpsc;

    let state_snapshot = engine.borrow().state.clone();
    let rules_snapshot = engine.borrow().rules.clone();

    // std::sync::mpsc works on any glib version (no `MainContext::channel` needed).
    let (tx, rx) = mpsc::channel::<Option<Vec<Move>>>();
    let rx = Mutex::new(rx);

    std::thread::spawn(move || {
        let sol = openpatience::core::solver::solve_from_state(
            &state_snapshot,
            &rules_snapshot,
            200_000,
        );
        // Ignore send error — receiver only drops on app exit.
        tx.send(sol).ok();
    });

    // Poll the channel from the GTK main loop every 100 ms.
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        match rx.lock().unwrap().try_recv() {
            Ok(sol) => {
                if let Some(s) = sol {
                    let mut e = engine.borrow_mut();
                    // Only adopt if the user hasn't moved yet.
                    if e.solution.is_none() && e.state.waste.is_empty() && e.state.score == 0 {
                        e.solution = Some(s);
                    }
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        }
    });
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
    // Debounce: prevents GestureClick + GestureDrag (or double-fired Button::clicked)
    // from both firing for one physical click. 50 ms is invisible to the user
    // but catches both-handlers-fire-in-same-tick.
    let last_action: Rc<Cell<std::time::Instant>> =
        Rc::new(Cell::new(std::time::Instant::now()));

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
                // Stock / Foundation are not drag sources — GestureClick handles them.
                // (If we also drew here, we'd get two draws per stock click.)
                if matches!(pile, PileRef::Stock | PileRef::Foundation(_)) {
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
        let last_action = last_action.clone();
        move |_g, offset_x, offset_y| {
            // Debounce: GestureClick may have fired for the same click
            let now = std::time::Instant::now();
            if now.duration_since(last_action.get()).as_millis() < 50 {
                *drag.borrow_mut() = None;
                return;
            }
            last_action.set(now);
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
            } else if drag.borrow().is_none() {
                // Drag gesture claimed the sequence but no drag state was set
                // (invalid target). Deselect silently — GestureClick was denied,
                // so this is the only handler for the sequence.
                *selection.borrow_mut() = None;
            }
            area.queue_draw();
        }
    });
    area.add_controller(drag_gesture);

    // ---------- Click handler ----------
    // Fires ONLY for pure clicks (no drag). If a drag happens, GestureDrag claims
    // the sequence and GestureClick's `released` is denied by GTK — no double-handling.
    let click = gtk::GestureClick::new();
    click.connect_released({
        let engine = engine.clone();
        let drag = drag.clone();
        let selection = selection.clone();
        let dims = dims.clone();
        let area = area.clone();
        let last_action = last_action.clone();
        move |_g, _n, x, y| {
            if drag.borrow().is_some() { return; }
            // Debounce: GestureDrag may have fired for the same click
            let now = std::time::Instant::now();
            if now.duration_since(last_action.get()).as_millis() < 50 {
                return;
            }
            last_action.set(now);

            let (w, h) = dims.get();
            let (scale, off_x, off_y) = compute_layout(&engine.borrow(), w, h);
            let bw = base_w(&engine.borrow());
            let hit = { let e = engine.borrow(); hit_test(&e.state, bw, scale, off_x, off_y, x, y) };

            match hit {
                Some((PileRef::Stock, _)) => {
                    let mut e = engine.borrow_mut();
                    if !e.state.stock.is_empty() { e.apply(Move::Draw); }
                    else { e.apply(Move::Recycle); }
                    *selection.borrow_mut() = None;
                }
                Some((PileRef::Tableau(t), idx)) => {
                    // Flip face-down top card on click
                    let e = engine.borrow();
                    let pile = &e.state.tableau[t as usize];
                    let is_top = (idx as usize) + 1 == pile.len();
                    let is_face_down = pile.get(idx as usize).map(|p| !p.face_up).unwrap_or(false);
                    drop(e);

                    if is_top && is_face_down {
                        engine.borrow_mut().apply(Move::FlipTableau(t));
                        *selection.borrow_mut() = None;
                    } else {
                        handle_select(&engine, &selection, PileRef::Tableau(t), idx);
                    }
                }
                Some((pile, idx)) => {
                    handle_select(&engine, &selection, pile, idx);
                }
                None => {
                    *selection.borrow_mut() = None;
                }
            }
            area.queue_draw();
        }
    });
    area.add_controller(click);

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
                    e.ensure_solution(50_000);
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
            spawn_background_solve(engine.clone());
        }
    });
    header.pack_start(&new_btn);

    let hint_btn = gtk::Button::builder()
        .icon_name("dialog-question-symbolic").tooltip_text("Hint (H)").build();
    hint_btn.connect_clicked({
        let engine = engine.clone(); let area = area.clone();
        let last_action = last_action.clone();
        move |_| {
            let now = std::time::Instant::now();
            if now.duration_since(last_action.get()).as_millis() < 50 {
                return;
            }
            last_action.set(now);
            let mut e = engine.borrow_mut();
            e.ensure_solution(50_000);
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
            spawn_background_solve(engine.clone());
        }
    });
    header.pack_start(&mode_drop);

    let about_btn = gtk::Button::builder()
        .icon_name("help-about-symbolic").tooltip_text("About Open Solitaire").build();
    header.pack_end(&about_btn);

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

    // About button
    {
        let win = window.clone();
        about_btn.connect_clicked(move |_| show_about_dialog(&win));
    }

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
                let _ = &mi;
                *eng.borrow_mut() = new_engine(mi.get(), new_diff);
                area2.queue_draw();
                spawn_background_solve(eng.clone());
            });
        });
    }

    window.set_titlebar(Some(&header));
    window.set_child(Some(&area));
    window.present();

    // Kick off background solving — window is already interactive.
    spawn_background_solve(engine.clone());
}


fn show_about_dialog(parent: &gtk::ApplicationWindow) {
    let about = gtk::Window::builder()
        .title("About Open Solitaire")
        .transient_for(parent)
        .modal(true)
        .default_width(440)
        .default_height(480)
        .resizable(false)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 6);
    vbox.set_margin_top(28);
    vbox.set_margin_bottom(24);
    vbox.set_margin_start(32);
    vbox.set_margin_end(32);

    let title = gtk::Label::new(None);
    title.set_markup("<span size='xx-large' weight='bold'>Open Solitaire</span>");
    vbox.append(&title);

    let subtitle = gtk::Label::new(Some("Beautiful, offline-first Solitaire for Linux"));
    subtitle.add_css_class("dim-label");
    vbox.append(&subtitle);

    vbox.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let version = gtk::Label::new(Some(&format!("Version {}", env!("CARGO_PKG_VERSION"))));
    vbox.append(&version);

    vbox.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let dev_label = gtk::Label::new(None);
    dev_label.set_markup("<span weight='bold'>Developer</span>");
    vbox.append(&dev_label);

    let dev_name = gtk::Label::new(None);
    dev_name.set_markup("<span size='large'>Abdelkader Berassil</span>");
    vbox.append(&dev_name);

    let dev_role = gtk::Label::new(Some("Computer Science Engineer · Algeria"));
    dev_role.add_css_class("dim-label");
    vbox.append(&dev_role);

    vbox.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let lic_label = gtk::Label::new(None);
    lic_label.set_markup("<span weight='bold'>License</span>");
    vbox.append(&lic_label);

    let lic_text = gtk::Label::new(Some("GNU General Public License v3.0 or later"));
    vbox.append(&lic_text);

    let built = gtk::Label::new(Some("Built with Rust · GTK4 · Cairo"));
    built.add_css_class("dim-label");
    built.set_margin_top(12);
    vbox.append(&built);

    let close = gtk::Button::with_label("Close");
    close.add_css_class("suggested-action");
    close.set_halign(gtk::Align::Center);
    close.set_margin_top(16);
    vbox.append(&close);

    about.set_child(Some(&vbox));

    let about_c = about.clone();
    close.connect_clicked(move |_| about_c.close());

    about.present();
}


fn handle_select(
    engine: &Rc<std::cell::RefCell<Engine>>,
    selection: &Rc<std::cell::RefCell<Option<Selection>>>,
    pile: PileRef,
    idx: u8,
) {
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
}
