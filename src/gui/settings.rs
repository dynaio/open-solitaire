use gtk4 as gtk;
use gtk::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

use openpatience::core::rules::{Redeals, RuleSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,    // draw 1, infinite redeals
    Normal,  // draw 3, infinite redeals
    Hard,    // draw 3, no redeals
}

impl Difficulty {
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self { Difficulty::Easy => "Easy", Difficulty::Normal => "Normal", Difficulty::Hard => "Hard" }
    }
}

#[allow(dead_code)]
pub struct GameConfig {
    pub difficulty: Rc<Cell<Difficulty>>,
}

pub fn apply_difficulty(rule: &mut RuleSet, d: Difficulty) {
    match d {
        Difficulty::Easy => { rule.stock.draw = 1; rule.stock.redeals = Redeals::Infinite; }
        Difficulty::Normal => { rule.stock.draw = 3; rule.stock.redeals = Redeals::Infinite; }
        Difficulty::Hard => { rule.stock.draw = 3; rule.stock.redeals = Redeals::Count(0); }
    }
}

pub fn show_settings_dialog(
    parent: &impl IsA<gtk::Window>,
    rule: &RuleSet,
    difficulty: Rc<Cell<Difficulty>>,
    on_apply: impl Fn(Difficulty) + 'static,
) {
    let win = gtk::Window::builder()
        .title("Settings — Open Solitaire")
        .transient_for(parent)
        .modal(true)
        .default_width(340)
        .default_height(260)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 12);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);

    let title = gtk::Label::new(Some("Difficulty"));
    title.set_xalign(0.0);
    title.add_css_class("title-4");
    vbox.append(&title);

    let hint = gtk::Label::new(Some(
        "Easy — draw 1, infinite redeals. Great for learning.\n\
         Normal — draw 3, infinite redeals. The classic challenge.\n\
         Hard — draw 3, no redeals. Win or start over."
    ));
    hint.set_xalign(0.0);
    hint.set_wrap(true);
    vbox.append(&hint);

    let easy = gtk::CheckButton::with_label("Easy");
    let normal = gtk::CheckButton::with_label("Normal");
    let hard = gtk::CheckButton::with_label("Hard");
    normal.set_group(Some(&easy));
    hard.set_group(Some(&easy));
    match difficulty.get() {
        Difficulty::Easy => easy.set_active(true),
        Difficulty::Normal => normal.set_active(true),
        Difficulty::Hard => hard.set_active(true),
    }
    vbox.append(&easy);
    vbox.append(&normal);
    vbox.append(&hard);

    vbox.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let info = gtk::Label::new(Some(&format!("Current mode: {}", rule.name)));
    info.set_xalign(0.0);
    vbox.append(&info);

    let btns = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    btns.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Cancel");
    let apply = gtk::Button::with_label("Apply");
    apply.add_css_class("suggested-action");
    btns.append(&cancel);
    btns.append(&apply);
    vbox.append(&btns);

    win.set_child(Some(&vbox));

    let win_c = win.clone();
    cancel.connect_clicked(move |_| win_c.close());

    let win_a = win.clone();
    let easy_c = easy.clone();
    let normal_c = normal.clone();
    let diff = difficulty.clone();
    apply.connect_clicked(move |_| {
        let chosen = if easy_c.is_active() { Difficulty::Easy }
                     else if normal_c.is_active() { Difficulty::Normal }
                     else { Difficulty::Hard };
        diff.set(chosen);
        on_apply(chosen);
        win_a.close();
    });

    win.present();
}
