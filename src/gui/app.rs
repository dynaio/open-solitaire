use std::cell::Cell;
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use openpatience::core::engine::Engine;
use openpatience::core::moves::Move;
use openpatience::core::rules::RuleSet;
use openpatience::core::state::PileRef;

use super::board::{
    base_w, compute_layout, draw_board, draw_victory_animation, hit_test, DragState, Selection,
    VictoryAnimation,
};
use super::settings::{apply_difficulty, show_settings_dialog, Difficulty};

const APP_ID: &str = "io.github.abdelkader.OpenSolitaire";

const MODES: &[(&str, &str)] = &[
    ("Klondike", include_str!("../../assets/rules/klondike.json")),
    (
        "Klondike ×3",
        include_str!("../../assets/rules/klondike-draw3.json"),
    ),
    ("FreeCell", include_str!("../../assets/rules/freecell.json")),
    (
        "Spider (1 Suit)",
        include_str!("../../assets/rules/spider-1suit.json"),
    ),
    (
        "Spider (4 Suits)",
        include_str!("../../assets/rules/spider-4suit.json"),
    ),
    ("Yukon", include_str!("../../assets/rules/yukon.json")),
    ("Canfield", include_str!("../../assets/rules/canfield.json")),
];

fn new_engine(mode_idx: usize, difficulty: Difficulty) -> Engine {
    let mut rule = RuleSet::from_json(MODES[mode_idx].1).expect("rule");
    // Only apply difficulty to Klondike variants (modes 0 and 1)
    if mode_idx < 2 {
        apply_difficulty(&mut rule, difficulty);
    }
    // Random seed from system time
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);
    Engine::new(rule, seed)
}

/// Solve in a background thread; attach the result to the Engine when done.
/// The window is fully interactive while this runs — user won't notice.
fn spawn_background_solve(engine: Rc<std::cell::RefCell<Engine>>) {
    use gtk::glib;
    use std::sync::mpsc;
    use std::sync::Mutex;

    let state_snapshot = engine.borrow().state.clone();
    let rules_snapshot = engine.borrow().rules.clone();

    let (tx, rx) = mpsc::channel::<Option<Vec<Move>>>();
    let rx = Mutex::new(rx);

    std::thread::spawn(move || {
        let sol =
            openpatience::core::solver::solve_from_state(&state_snapshot, &rules_snapshot, 200_000);
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

fn check_win(
    engine: &Rc<std::cell::RefCell<Engine>>,
    window: &gtk::ApplicationWindow,
    area: &gtk::DrawingArea,
    move_count: u32,
    start_time: std::time::Instant,
    victory_shown: &Cell<bool>,
    victory_animation: &Rc<std::cell::RefCell<Option<VictoryAnimation>>>,
) {
    if !victory_shown.get() && engine.borrow().state.is_won() {
        victory_shown.set(true);

        let score = engine.borrow().state.score;
        let secs = start_time.elapsed().as_secs();

        // Snapshot all foundation cards before starting the animation.
        *victory_animation.borrow_mut() = Some(VictoryAnimation::new(&engine.borrow().state));

        // Refresh the drawing area at approximately 60 FPS.
        let animation = victory_animation.clone();
        let area = area.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            let finished = animation
                .borrow()
                .as_ref()
                .map(VictoryAnimation::is_finished)
                .unwrap_or(true);

            if finished {
                animation.borrow_mut().take();
                area.queue_draw();
                glib::ControlFlow::Break
            } else {
                area.queue_draw();
                glib::ControlFlow::Continue
            }
        });

        show_win_dialog(window, score, move_count, secs);
    }
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
    let selection: Rc<std::cell::RefCell<Option<Selection>>> =
        Rc::new(std::cell::RefCell::new(None));
    let hover: Rc<std::cell::RefCell<Option<(PileRef, u8)>>> =
        Rc::new(std::cell::RefCell::new(None));
    let drag: Rc<std::cell::RefCell<Option<DragState>>> = Rc::new(std::cell::RefCell::new(None));
    let dims = Rc::new(Cell::new((1000.0f64, 800.0f64)));
    let last_action: Rc<Cell<std::time::Instant>> = Rc::new(Cell::new(std::time::Instant::now()));

    // Score / move tracking
    let move_count: Rc<Cell<u32>> = Rc::new(Cell::new(0));
    let start_time: Rc<Cell<std::time::Instant>> = Rc::new(Cell::new(std::time::Instant::now()));
    let victory_shown = Rc::new(Cell::new(false));
    let victory_animation: Rc<std::cell::RefCell<Option<VictoryAnimation>>> =
        Rc::new(std::cell::RefCell::new(None));

    let area = gtk::DrawingArea::new();
    area.set_hexpand(true);
    area.set_vexpand(true);

    area.set_draw_func({
        let engine = engine.clone();
        let selection = selection.clone();
        let hover = hover.clone();
        let drag = drag.clone();
        let dims = dims.clone();
        let victory_animation = victory_animation.clone();
        move |_area, cr, w, h| {
            dims.set((w as f64, h as f64));
            draw_board(
                cr,
                w as f64,
                h as f64,
                &engine.borrow(),
                &selection.borrow(),
                &hover.borrow(),
                &drag.borrow(),
            );

            if let Some(animation) = victory_animation.borrow().as_ref() {
                draw_victory_animation(cr, w as f64, h as f64, animation);
            }
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
            let hit = {
                let e = engine.borrow();
                hit_test(&e.state, bw, scale, off_x, off_y, x, y)
            };
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
            let hit = {
                let e = engine.borrow();
                hit_test(&e.state, bw, scale, off_x, off_y, start_x, start_y)
            };
            if let Some((pile, idx)) = hit {
                // Stock / Foundation are not drag sources — GestureClick handles them.
                if matches!(pile, PileRef::Stock | PileRef::Foundation(_)) {
                    return;
                }
                // Only start drag from movable face-up sources
                let e = engine.borrow();
                let cards: Vec<_> = match pile {
                    PileRef::Tableau(t) => {
                        let p = &e.state.tableau[t as usize];
                        if idx as usize >= p.len() || !p[idx as usize].face_up {
                            return;
                        }
                        p[idx as usize..].iter().map(|pc| pc.card).collect()
                    }
                    PileRef::Waste => match e.state.waste.last() {
                        Some(c) => vec![*c],
                        None => return,
                    },
                    PileRef::FreeCell(f) => match e.state.free_cells[f as usize] {
                        Some(c) => vec![c],
                        None => return,
                    },
                    _ => return,
                };
                drop(e);
                *drag.borrow_mut() = Some(DragState {
                    pile,
                    from_idx: idx,
                    cards,
                    start_x,
                    start_y,
                    mouse_x: start_x,
                    mouse_y: start_y,
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
        let move_count = move_count.clone();
        let _start_time = start_time.clone();
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
            let target = hit_test(
                &engine.borrow().state,
                bw,
                scale,
                off_x,
                off_y,
                end_x,
                end_y,
            );

            let distance = (offset_x.powi(2) + offset_y.powi(2)).sqrt();

            if let Some(d) = dragged {
                if distance > 8.0 {
                    if let Some((to_pile, _)) = target {
                        let m = Move::Move {
                            from: d.pile,
                            from_idx: d.from_idx,
                            to: to_pile,
                        };
                        if engine.borrow_mut().apply(m) {
                            move_count.set(move_count.get() + 1);
                        }
                        *selection.borrow_mut() = None;
                    }
                } else {
                    // Short drag = click: use the normal card interaction logic.
                    let did_move = if selection.borrow().is_none()
                        && try_auto_foundation(&engine, d.pile, d.from_idx)
                    {
                        *selection.borrow_mut() = None;
                        true
                    } else {
                        handle_select(&engine, &selection, d.pile, d.from_idx)
                    };

                    if did_move {
                        move_count.set(move_count.get() + 1);
                    }
                }
            } else if drag.borrow().is_none() {
                *selection.borrow_mut() = None;
            }
            area.queue_draw();
        }
    });
    area.add_controller(drag_gesture);

    // ---------- Click handler ----------
    let click = gtk::GestureClick::new();
    click.connect_released({
        let engine = engine.clone();
        let drag = drag.clone();
        let selection = selection.clone();
        let dims = dims.clone();
        let area = area.clone();
        let last_action = last_action.clone();
        let move_count = move_count.clone();
        let _start_time = start_time.clone();
        move |_g, n_press, x, y| {
            if drag.borrow().is_some() {
                return;
            }
            // Debounce: GestureDrag may have fired for the same click
            let now = std::time::Instant::now();
            if now.duration_since(last_action.get()).as_millis() < 50 {
                return;
            }
            last_action.set(now);

            let (w, h) = dims.get();
            let (scale, off_x, off_y) = compute_layout(&engine.borrow(), w, h);
            let bw = base_w(&engine.borrow());
            let hit = {
                let e = engine.borrow();
                hit_test(&e.state, bw, scale, off_x, off_y, x, y)
            };

            // Double-click: try a foundation first, then a legal tableau destination.
            if n_press >= 2 {
                if let Some((pile, idx)) = hit {
                    if auto_move_card(&engine, pile, idx) {
                        *selection.borrow_mut() = None;
                        move_count.set(move_count.get() + 1);
                        area.queue_draw();
                        return;
                    }
                }
            }

            let mut did_move = false;
            match hit {
                Some((PileRef::Stock, _)) => {
                    let mut e = engine.borrow_mut();
                    did_move = if !e.state.stock.is_empty() {
                        e.apply(Move::Draw)
                    } else {
                        e.apply(Move::Recycle)
                    };
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
                        did_move = true;
                    } else if selection.borrow().is_none()
                        && try_auto_foundation(&engine, PileRef::Tableau(t), idx)
                    {
                        *selection.borrow_mut() = None;
                        did_move = true;
                    } else {
                        did_move = handle_select(&engine, &selection, PileRef::Tableau(t), idx);
                    }
                }
                Some((pile, idx)) => {
                    if selection.borrow().is_none() && try_auto_foundation(&engine, pile, idx) {
                        *selection.borrow_mut() = None;
                        did_move = true;
                    } else {
                        did_move = handle_select(&engine, &selection, pile, idx);
                    }
                }
                None => {
                    *selection.borrow_mut() = None;
                }
            }
            if did_move {
                move_count.set(move_count.get() + 1);
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
        let move_count = move_count.clone();
        move |_ctrl, keyval, _keycode, _state| {
            let name = keyval.name().map(|s| s.to_string()).unwrap_or_default();
            let mut e = engine.borrow_mut();
            let handled = match name.as_str() {
                "u" | "U" => {
                    e.undo();
                    true
                }
                "h" | "H" => {
                    e.ensure_solution(50_000);
                    if let Some(m) = e.hint() {
                        if e.apply(m) {
                            drop(e);
                            move_count.set(move_count.get() + 1);
                            if handled_guard(&area) {}
                            area.queue_draw();
                            return glib::Propagation::Stop;
                        }
                    }
                    true
                }
                "f" | "F" => {
                    e.auto_foundation_all();
                    true
                }
                "d" | "D" => {
                    e.apply(Move::Draw);
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

    // Header
    let header = adw::HeaderBar::new();

    let home_btn = gtk::Button::builder()
        .icon_name("go-home-symbolic")
        .tooltip_text("Home")
        .build();

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
        .tooltip_text("New game")
        .build();

    let hint_btn = gtk::Button::builder()
        .icon_name("dialog-question-symbolic")
        .tooltip_text("Hint (H)")
        .build();
    hint_btn.connect_clicked({
        let engine = engine.clone();
        let area = area.clone();
        let last_action = last_action.clone();
        let move_count = move_count.clone();
        move |_| {
            let now = std::time::Instant::now();
            if now.duration_since(last_action.get()).as_millis() < 50 {
                return;
            }
            last_action.set(now);
            let mut e = engine.borrow_mut();
            e.ensure_solution(50_000);
            if let Some(m) = e.hint() {
                if e.apply(m) {
                    drop(e);
                    move_count.set(move_count.get() + 1);
                    area.queue_draw();
                    return;
                }
            }
            drop(e);
            area.queue_draw();
        }
    });
    header.pack_start(&hint_btn);

    // Score label
    let score_label = gtk::Label::new(Some("Score: 0  Moves: 0  00:00"));
    score_label.add_css_class("heading");
    score_label.set_margin_start(8);
    score_label.set_margin_end(8);
    header.pack_start(&score_label);

    let mode_strings: Vec<&str> = MODES.iter().map(|(n, _)| *n).collect();
    let mode_drop = gtk::DropDown::from_strings(&mode_strings);
    mode_drop.set_selected(0);
    mode_drop.set_tooltip_text(Some("Game mode"));

    let about_btn = gtk::Button::builder()
        .icon_name("help-about-symbolic")
        .tooltip_text("About Open Solitaire")
        .build();
    header.pack_end(&about_btn);

    let auto_btn = gtk::Button::builder()
        .icon_name("go-up-symbolic")
        .tooltip_text("Auto-foundation (F)")
        .build();
    auto_btn.connect_clicked({
        let engine = engine.clone();
        let area = area.clone();
        move |_| {
            engine.borrow_mut().auto_foundation_all();
            area.queue_draw();
        }
    });
    header.pack_end(&auto_btn);

    let settings_btn = gtk::Button::builder()
        .icon_name("emblem-system-symbolic")
        .tooltip_text("Settings")
        .build();
    header.pack_end(&settings_btn);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Open Solitaire")
        .default_width(1000)
        .default_height(800)
        .build();

    // Main navigation.
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(250);
    stack.add_named(&area, Some("game"));

    // Entrance screen.
    let home_page = gtk::Box::new(gtk::Orientation::Vertical, 22);
    home_page.set_halign(gtk::Align::Center);
    home_page.set_valign(gtk::Align::Center);
    home_page.set_margin_top(36);
    home_page.set_margin_bottom(36);
    home_page.set_margin_start(32);
    home_page.set_margin_end(32);

    let welcome = gtk::Label::new(None);
    welcome.set_markup("<span size='32000' weight='bold'>♠ Open Solitaire</span>");
    welcome.set_halign(gtk::Align::Center);
    home_page.append(&welcome);

    let subtitle = gtk::Label::new(Some(
        "Seven classic solitaire games. Your table, your rules.",
    ));
    subtitle.add_css_class("dim-label");
    subtitle.set_halign(gtk::Align::Center);
    home_page.append(&subtitle);

    let choose = gtk::Label::new(Some("CHOOSE YOUR GAME"));
    choose.add_css_class("heading");
    choose.set_halign(gtk::Align::Center);
    home_page.append(&choose);

    let descriptions = [
        "The timeless solitaire classic",
        "Draw three cards at a time",
        "Four free cells. Plan every move",
        "A relaxing one-suit challenge",
        "The full four-suit Spider challenge",
        "Move any face-up sequence",
        "A reserve pile and changing foundations",
    ];

    let mode_grid = gtk::Grid::new();
    mode_grid.set_row_spacing(12);
    mode_grid.set_column_spacing(12);
    mode_grid.set_halign(gtk::Align::Center);
    mode_grid.set_column_homogeneous(true);

    let game_controls: Vec<gtk::Widget> = vec![
        home_btn.clone().upcast(),
        undo_btn.clone().upcast(),
        new_btn.clone().upcast(),
        hint_btn.clone().upcast(),
        score_label.clone().upcast(),
        mode_drop.clone().upcast(),
        about_btn.clone().upcast(),
        auto_btn.clone().upcast(),
        settings_btn.clone().upcast(),
    ];

    for control in &game_controls {
        control.set_visible(false);
    }

    for (index, (name, _)) in MODES.iter().enumerate() {
        let button = gtk::Button::new();
        button.add_css_class("card");
        button.set_hexpand(true);
        button.set_size_request(250, 92);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(16);
        content.set_margin_end(16);

        let title = gtk::Label::new(Some(name));
        title.add_css_class("title-3");
        title.set_halign(gtk::Align::Start);

        let description = gtk::Label::new(Some(descriptions[index]));
        description.add_css_class("dim-label");
        description.set_halign(gtk::Align::Start);

        content.append(&title);
        content.append(&description);
        button.set_child(Some(&content));

        mode_grid.attach(&button, (index % 2) as i32, (index / 2) as i32, 1, 1);

        button.connect_clicked({
            let mode_drop = mode_drop.clone();
            let new_btn = new_btn.clone();
            let stack = stack.clone();
            let controls = game_controls.clone();

            move |_| {
                let same_mode = mode_drop.selected() == index as u32;
                mode_drop.set_selected(index as u32);

                if same_mode {
                    new_btn.emit_clicked();
                }

                for control in &controls {
                    control.set_visible(true);
                }

                stack.set_visible_child_name("game");
            }
        });
    }

    home_page.append(&mode_grid);

    let footer = gtk::Label::new(Some("Free · Open source · Offline · No ads"));
    footer.add_css_class("dim-label");
    footer.set_halign(gtk::Align::Center);
    home_page.append(&footer);

    stack.add_named(&home_page, Some("home"));
    stack.set_visible_child_name("home");

    home_btn.connect_clicked({
        let stack = stack.clone();
        let controls = game_controls.clone();

        move |_| {
            stack.set_visible_child_name("home");
            for control in &controls {
                control.set_visible(false);
            }
        }
    });

    // Wire up new_btn now that window exists
    new_btn.connect_clicked({
        let engine = engine.clone();
        let mode_idx = mode_idx.clone();
        let difficulty = difficulty.clone();
        let area = area.clone();
        let move_count = move_count.clone();
        let start_time = start_time.clone();
        let victory_shown = victory_shown.clone();
        let victory_animation = victory_animation.clone();
        move |_| {
            *engine.borrow_mut() = new_engine(mode_idx.get(), difficulty.get());
            move_count.set(0);
            start_time.set(std::time::Instant::now());
            victory_shown.set(false);
            victory_animation.borrow_mut().take();
            area.queue_draw();
            spawn_background_solve(engine.clone());
        }
    });
    header.pack_start(&new_btn);

    // Wire up mode drop
    mode_drop.connect_selected_notify({
        let engine = engine.clone();
        let mode_idx = mode_idx.clone();
        let difficulty = difficulty.clone();
        let area = area.clone();
        let move_count = move_count.clone();
        let start_time = start_time.clone();
        let victory_shown = victory_shown.clone();
        let victory_animation = victory_animation.clone();
        move |dd| {
            let idx = dd.selected() as usize;
            if idx >= MODES.len() {
                return;
            }
            mode_idx.set(idx);
            *engine.borrow_mut() = new_engine(idx, difficulty.get());
            move_count.set(0);
            start_time.set(std::time::Instant::now());
            victory_shown.set(false);
            victory_animation.borrow_mut().take();
            area.queue_draw();
            spawn_background_solve(engine.clone());
        }
    });
    header.pack_start(&mode_drop);

    // Score refresh timer — update every second
    glib::timeout_add_local(std::time::Duration::from_secs(1), {
        let engine = engine.clone();
        let score_label = score_label.clone();
        let move_count = move_count.clone();
        let start_time = start_time.clone();
        move || {
            let e = engine.borrow();
            let elapsed = start_time.get().elapsed().as_secs();
            let score = e.state.score;
            let moves = move_count.get();
            score_label.set_text(&format!(
                "Score: {}  Moves: {}  {:02}:{:02}",
                score,
                moves,
                elapsed / 60,
                elapsed % 60
            ));
            glib::ControlFlow::Continue
        }
    });

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
        let move_count = move_count.clone();
        let start_time = start_time.clone();
        let victory_shown = victory_shown.clone();
        let victory_animation = victory_animation.clone();
        settings_btn.connect_clicked(move |_| {
            let rule = engine.borrow().rules.clone();
            let diff = difficulty.clone();
            let eng = engine.clone();
            let area2 = area.clone();
            let mi = mode_idx.clone();
            let mc = move_count.clone();
            let st = start_time.clone();
            let victory = victory_shown.clone();
            let animation = victory_animation.clone();
            show_settings_dialog(&win, &rule, diff, move |new_diff| {
                let _ = &mi;
                *eng.borrow_mut() = new_engine(mi.get(), new_diff);
                mc.set(0);
                st.set(std::time::Instant::now());
                victory.set(false);
                animation.borrow_mut().take();
                area2.queue_draw();
                spawn_background_solve(eng.clone());
            });
        });
    }

    // Win-check timer (every 500ms, lightweight)
    glib::timeout_add_local(std::time::Duration::from_millis(500), {
        let engine = engine.clone();
        let window = window.clone();
        let move_count = move_count.clone();
        let start_time = start_time.clone();
        let victory_shown = victory_shown.clone();
        let victory_animation = victory_animation.clone();
        let area = area.clone();
        move || {
            check_win(
                &engine,
                &window,
                &area,
                move_count.get(),
                start_time.get(),
                &victory_shown,
                &victory_animation,
            );
            glib::ControlFlow::Continue
        }
    });

    header.pack_start(&home_btn);
    window.set_titlebar(Some(&header));
    window.set_child(Some(&stack));
    window.present();
}

fn show_win_dialog(parent: &gtk::ApplicationWindow, score: i32, moves: u32, seconds: u64) {
    let win = gtk::Window::builder()
        .title("🎉 You Won!")
        .transient_for(parent)
        .modal(true)
        .default_width(380)
        .default_height(300)
        .resizable(false)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 12);
    vbox.set_margin_top(32);
    vbox.set_margin_bottom(28);
    vbox.set_margin_start(32);
    vbox.set_margin_end(32);

    let title = gtk::Label::new(None);
    title.set_markup("<span size='xx-large' weight='bold'>🎉 Congratulations!</span>");
    vbox.append(&title);

    let subtitle = gtk::Label::new(Some("You completed the game!"));
    subtitle.add_css_class("title-3");
    vbox.append(&subtitle);

    vbox.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let stats = gtk::Label::new(None);
    stats.set_markup(&format!(
        "<b>Score:</b> {}\n<b>Moves:</b> {}\n<b>Time:</b> {:02}:{:02}",
        score,
        moves,
        seconds / 60,
        seconds % 60
    ));
    stats.set_justify(gtk::Justification::Center);
    vbox.append(&stats);

    let btns = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    btns.set_halign(gtk::Align::Center);
    btns.set_margin_top(8);

    let new_btn = gtk::Button::with_label("New Game");
    new_btn.add_css_class("suggested-action");
    let close_btn = gtk::Button::with_label("Close");
    btns.append(&close_btn);
    btns.append(&new_btn);
    vbox.append(&btns);

    win.set_child(Some(&vbox));

    let win_c = win.clone();
    close_btn.connect_clicked(move |_| win_c.close());
    let win_n = win.clone();
    new_btn.connect_clicked(move |_| win_n.close());

    win.present();
}

fn show_about_dialog(parent: &gtk::ApplicationWindow) {
    let dialog = gtk::Window::builder()
        .title("About Open Solitaire")
        .transient_for(parent)
        .modal(true)
        .default_width(500)
        .default_height(560)
        .resizable(false)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);

    // Header banner
    let header_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    header_box.set_margin_top(24);
    header_box.set_margin_bottom(16);
    let title = gtk::Label::new(None);
    title.set_markup("<span size='xx-large' weight='bold'>♠ Open Solitaire</span>");
    header_box.append(&title);
    let ver = gtk::Label::new(Some(&format!("Version {}", env!("CARGO_PKG_VERSION"))));
    ver.add_css_class("dim-label");
    header_box.append(&ver);
    vbox.append(&header_box);
    vbox.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    // Notebook (tabs)
    let notebook = gtk::Notebook::new();
    notebook.set_margin_top(8);
    notebook.set_margin_start(0);
    notebook.set_margin_end(0);
    notebook.set_vexpand(true);

    // Tab 1: About
    let about_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    about_box.set_margin_top(16);
    about_box.set_margin_bottom(16);
    about_box.set_margin_start(24);
    about_box.set_margin_end(24);

    let dev_title = gtk::Label::new(None);
    dev_title.set_markup("<b>Developer</b>");
    dev_title.set_xalign(0.0);
    about_box.append(&dev_title);
    let dev_name = gtk::Label::new(None);
    dev_name.set_markup("<span size='large'>Abdelkader Berassil</span>");
    dev_name.set_xalign(0.0);
    about_box.append(&dev_name);
    let dev_role = gtk::Label::new(Some("Computer Science Engineer · Algeria"));
    dev_role.add_css_class("dim-label");
    dev_role.set_xalign(0.0);
    about_box.append(&dev_role);

    about_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let lic_title = gtk::Label::new(None);
    lic_title.set_markup("<b>License</b>");
    lic_title.set_xalign(0.0);
    about_box.append(&lic_title);
    let lic = gtk::Label::new(Some("GNU General Public License v3.0 or later\nThis is free software — you are free to use, copy, modify, and distribute it."));
    lic.set_wrap(true);
    lic.set_xalign(0.0);
    about_box.append(&lic);

    about_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let tech_title = gtk::Label::new(None);
    tech_title.set_markup("<b>Technology</b>");
    tech_title.set_xalign(0.0);
    about_box.append(&tech_title);
    let tech = gtk::Label::new(Some("Built with Rust · GTK 4 · libadwaita · Cairo\nRuns entirely offline. No dependencies on cloud services."));
    tech.set_wrap(true);
    tech.set_xalign(0.0);
    about_box.append(&tech);

    notebook.append_page(&about_box, Some(&gtk::Label::new(Some("About"))));

    // Tab 2: Privacy Policy
    let priv_scroll = gtk::ScrolledWindow::new();
    priv_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    let priv_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    priv_box.set_margin_top(16);
    priv_box.set_margin_bottom(16);
    priv_box.set_margin_start(24);
    priv_box.set_margin_end(24);

    let priv_title = gtk::Label::new(None);
    priv_title.set_markup("<b>Privacy Policy</b>");
    priv_title.set_xalign(0.0);
    priv_box.append(&priv_title);

    let priv_text = gtk::Label::new(Some(
        "Open Solitaire is a fully offline application.\n\n\
        ✓ No data collection — We collect absolutely no personal data.\n\
        ✓ No network access — The app never connects to the internet.\n\
        ✓ No telemetry — No usage statistics are sent anywhere.\n\
        ✓ No tracking — No analytics, crash reporters, or beacons.\n\
        ✓ No accounts — No login, registration, or cloud sync.\n\
        ✓ No ads — This is free software with no advertising.\n\
        ✓ No third-party SDKs — No external libraries phone home.\n\n\
        Your game data (seed, score) is computed entirely in RAM \
        and discarded when you close the app. Nothing is written \
        to disk, the network, or any external service.\n\n\
        This policy is effective as of 2026 and applies to all \
        versions of Open Solitaire distributed under the GPL-3.0 license.",
    ));
    priv_text.set_wrap(true);
    priv_text.set_xalign(0.0);
    priv_text.set_selectable(true);
    priv_box.append(&priv_text);
    priv_scroll.set_child(Some(&priv_box));
    notebook.append_page(&priv_scroll, Some(&gtk::Label::new(Some("Privacy"))));

    // Tab 3: Game Rules
    let rules_scroll = gtk::ScrolledWindow::new();
    rules_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    let rules_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    rules_box.set_margin_top(16);
    rules_box.set_margin_bottom(16);
    rules_box.set_margin_start(24);
    rules_box.set_margin_end(24);

    let rules_text = gtk::Label::new(Some(
        "KLONDIKE — The classic solitaire. Build 4 foundation piles from Ace to King by suit. \
        In the tableau, build down in alternating colors. Only Kings go to empty columns.\n\n\
        KLONDIKE (DRAW 3) — Same as Klondike but draw 3 cards at once from the stock. Harder!\n\n\
        FREECELL — All 52 cards dealt face-up into 8 columns. Use 4 free cells as temporary \
        parking. Almost every deal is solvable.\n\n\
        SPIDER (1 SUIT) — Two decks, all Spades. Build 8 descending sequences King to Ace. \
        Complete sequences auto-move to foundations.\n\n\
        SPIDER (4 SUITS) — The ultimate challenge. Same as 1-suit but with all 4 suits.\n\n\
        YUKON — All cards dealt face-up. You can move any group of face-up cards regardless \
        of sequence order. Build foundations Ace to King by suit.\n\n\
        CANFIELD — A fast-paced game with a 13-card reserve pile. Draw 3, build foundations \
        starting from a random rank.\n\n\
        KEYBOARD SHORTCUTS\n\
        U — Undo last move\n\
        H — Hint (applies next best move)\n\
        F — Auto-foundation all possible cards\n\
        D — Draw from stock",
    ));
    rules_text.set_wrap(true);
    rules_text.set_xalign(0.0);
    rules_text.set_selectable(true);
    rules_box.append(&rules_text);
    rules_scroll.set_child(Some(&rules_box));
    notebook.append_page(&rules_scroll, Some(&gtk::Label::new(Some("Rules"))));

    vbox.append(&notebook);
    vbox.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let close_btn = gtk::Button::with_label("Close");
    close_btn.add_css_class("suggested-action");
    close_btn.set_halign(gtk::Align::Center);
    close_btn.set_margin_top(12);
    close_btn.set_margin_bottom(12);
    vbox.append(&close_btn);

    dialog.set_child(Some(&vbox));
    let dialog_c = dialog.clone();
    close_btn.connect_clicked(move |_| dialog_c.close());
    dialog.present();
}

/// Move a clicked top card to its foundation when legal.
fn try_auto_foundation(engine: &Rc<std::cell::RefCell<Engine>>, pile: PileRef, idx: u8) -> bool {
    let is_top = {
        let e = engine.borrow();
        match pile {
            PileRef::Tableau(t) => e
                .state
                .tableau
                .get(t as usize)
                .map(|column| {
                    idx as usize + 1 == column.len()
                        && column.last().map(|card| card.face_up).unwrap_or(false)
                })
                .unwrap_or(false),
            PileRef::Waste => !e.state.waste.is_empty() && idx as usize + 1 == e.state.waste.len(),
            PileRef::FreeCell(f) => e
                .state
                .free_cells
                .get(f as usize)
                .copied()
                .flatten()
                .is_some(),
            _ => false,
        }
    };

    is_top && engine.borrow_mut().apply(Move::AutoFoundation(pile))
}

/// Automatically place a double-clicked single card at a legal destination.
fn auto_move_card(engine: &Rc<std::cell::RefCell<Engine>>, pile: PileRef, idx: u8) -> bool {
    let from_idx = match pile {
        PileRef::Tableau(t) => {
            let e = engine.borrow();
            let Some(column) = e.state.tableau.get(t as usize) else {
                return false;
            };
            if idx as usize + 1 != column.len()
                || !column.last().map(|card| card.face_up).unwrap_or(false)
            {
                return false;
            }
            idx
        }
        PileRef::Waste => {
            let e = engine.borrow();
            if e.state.waste.is_empty() {
                return false;
            }
            (e.state.waste.len() - 1) as u8
        }
        PileRef::FreeCell(f) => {
            let e = engine.borrow();
            if e.state
                .free_cells
                .get(f as usize)
                .copied()
                .flatten()
                .is_none()
            {
                return false;
            }
            0
        }
        _ => return false,
    };

    // Prefer a foundation whenever the move is legal.
    if engine.borrow_mut().apply(Move::AutoFoundation(pile)) {
        return true;
    }

    // Otherwise, try legal tableau destinations.
    let columns = engine.borrow().state.tableau.len();
    for target in 0..columns {
        let movement = Move::Move {
            from: pile,
            from_idx,
            to: PileRef::Tableau(target as u8),
        };
        let mut e = engine.borrow_mut();
        if e.can(&movement) && e.apply(movement) {
            return true;
        }
    }

    false
}

fn handle_select(
    engine: &Rc<std::cell::RefCell<Engine>>,
    selection: &Rc<std::cell::RefCell<Option<Selection>>>,
    pile: PileRef,
    idx: u8,
) -> bool {
    let cur = Selection {
        pile,
        from_idx: idx,
    };

    // Copy the current selection first so the immutable RefCell borrow
    // is dropped before any mutable borrow occurs.
    let current = *selection.borrow();

    match current {
        Some(s) if s.pile == cur.pile && s.from_idx == cur.from_idx => {
            *selection.borrow_mut() = None;
            false
        }
        None => {
            *selection.borrow_mut() = Some(cur);
            false
        }
        Some(s) => {
            let movement = Move::Move {
                from: s.pile,
                from_idx: s.from_idx,
                to: pile,
            };

            if engine.borrow_mut().apply(movement) {
                *selection.borrow_mut() = None;
                true
            } else {
                *selection.borrow_mut() = Some(cur);
                false
            }
        }
    }
}

// Dummy helper to satisfy borrow checker in hint handler (unused body)
fn handled_guard(_area: &gtk::DrawingArea) -> bool {
    false
}
