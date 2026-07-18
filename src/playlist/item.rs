use gtk::gio;
use gtk::prelude::*;
use mutsumi::PlaylistItem;

pub trait PlaylistFileItem {
    fn from_file(file: &gio::File) -> PlaylistItem;
}

impl PlaylistFileItem for PlaylistItem {
    fn from_file(file: &gio::File) -> PlaylistItem {
        let uri = file.uri().to_string();

        PlaylistItem::with_full_uri(&uri)
    }
}
