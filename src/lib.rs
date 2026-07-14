mod app;
mod args;
mod css;
mod files;
mod macros;
mod playlist;
mod status;
mod window;

use gtk::{glib, prelude::*};

pub use app::FughettaApplication as Application;
pub use files::*;
pub use playlist::{PlayList, SourceActionRow};
pub use window::*;

pub const APP_ID: &str = "io.github.mutsumiuniverse.fughetta";
pub const APP_RESOURCE_PATH: &str = "/io/github/mutsumiuniverse/fughetta";
pub const CLIENT_ID: &str = "Fughetta";
pub const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() -> glib::ExitCode {
    gtk::gio::resources_register_include!("fughetta.gresource")
        .expect("Failed to register resources.");

    mutsumi::force_gl_renderer();

    gtk::glib::set_application_name(CLIENT_ID);

    Application::new().run()
}
