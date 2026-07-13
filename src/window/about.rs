use std::ops::Deref;

use adw::AboutDialog;

use crate::*;

pub struct FughettaAboutDialog(AboutDialog);

impl FughettaAboutDialog {
    pub fn new() -> Self {
        let dialog = AboutDialog::builder()
            .application_name(CLIENT_ID)
            .version(CLIENT_VERSION)
            .comments("A GTK4 frontend for MPV, embedded by wl-proxy, written in Rust.")
            .website("https://github.com/mutsumiuniverse/fughetta")
            .application_icon(APP_ID)
            .license_type(gtk::License::Gpl30)
            .copyright("© MutsumiUniverse")
            .build();

        dialog.add_credit_section(Some("Developers"), &["Inaha"]);

        Self(dialog)
    }
}

impl Default for FughettaAboutDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for FughettaAboutDialog {
    type Target = AboutDialog;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
