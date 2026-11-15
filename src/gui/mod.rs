pub mod app;
pub mod board;
pub mod render;
pub mod settings;

pub fn run() -> gtk4::glib::ExitCode {
    app::run()
}
