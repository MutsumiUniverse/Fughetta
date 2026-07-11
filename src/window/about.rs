use std::ops::Deref;

use adw::AboutDialog;

use crate::*;

pub struct FughettaAboutDialog(AboutDialog);

impl Default for FughettaAboutDialog {
    fn default() -> Self {
        Self(
            AboutDialog::builder()
                .application_name(CLIENT_ID)
                .version(CLIENT_VERSION)
                .comments("A GTK4 frontend for MPV, embedded by wl-proxy, written in Rust.")
                .website("https://github.com/mutsumiuniverse/fughetta")
                .application_icon("io.github.mutsumiuniverse.fughetta")
                .license_type(gtk::License::Gpl30)
                .build(),
        )
    }
}

impl Deref for FughettaAboutDialog {
    type Target = AboutDialog;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
