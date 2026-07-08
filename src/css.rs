const PLAYLIST_CSS: &str = include_str!("playlist.css");

pub fn init() {
    let display = gtk::gdk::Display::default().expect("Failed to get default display");

    let provider = gtk::CssProvider::new();
    provider.load_from_string(PLAYLIST_CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
