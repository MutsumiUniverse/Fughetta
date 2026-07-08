mod app;
mod css;
mod playlist;
mod status;
mod window;

use gtk::prelude::*;

pub use app::FughettaApplication as Application;
pub use playlist::{PlayList, SourceActionRow};
pub use window::*;

pub const APP_ID: &str = "io.github.mutsumiuniverse.fughetta";
pub const APP_RESOURCE_PATH: &str = "/io/github/mutsumiuniverse/fughetta";
pub const CLIENT_ID: &str = "Fughetta";
pub const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    gtk::gio::resources_register_include!("fughetta.gresource")
        .expect("Failed to register resources.");

    mutsumi::force_gl_renderer();

    gtk::glib::set_application_name(CLIENT_ID);

    Application::new().run_with_args::<&str>(&[]);
}
