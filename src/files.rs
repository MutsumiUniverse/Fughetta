use gtk::gio::File;
use std::sync::OnceLock;

pub static ARG_FILES: OnceLock<Vec<File>> = OnceLock::new();
