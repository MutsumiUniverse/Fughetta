use std::{
    cell::RefCell,
    rc::{self, Rc},
};

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{CompositeTemplate, glib};
use mutsumi::MutsumiPlayer;

use crate::PlayList;

mod imp {
    use mutsumi::MutsumiPlayer;

    use crate::PlayList;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/mutsumiuniverse/fughetta/ui/status.ui")]
    pub struct PlaceHolderStatus {
        pub playlist: glib::WeakRef<PlayList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaceHolderStatus {
        const NAME: &'static str = "PlaceHolderStatus";
        type Type = super::PlaceHolderStatus;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PlaceHolderStatus {}

    impl WidgetImpl for PlaceHolderStatus {}
    impl BinImpl for PlaceHolderStatus {}
}

glib::wrapper! {
    pub struct PlaceHolderStatus(ObjectSubclass<imp::PlaceHolderStatus>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl PlaceHolderStatus {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    fn on_restore_history_activated(&self, _button: &gtk::Button) {
        let Some(playlist) = self.imp().playlist.upgrade() else {
            return;
        };

        let dialog = adw::AlertDialog::new(Some("Not available yet"), None);
        dialog.set_width_request(500);
        dialog.add_response("cancel", "懂你意思");
        dialog.set_response_appearance("cancel", adw::ResponseAppearance::Destructive);
        dialog.set_close_response("cancel");
        dialog.present(Some(self));
    }

    #[template_callback]
    fn on_file_selector_activated(&self, _button: &gtk::Button) {
        let Some(playlist) = self.imp().playlist.upgrade() else {
            return;
        };

        playlist
            .imp()
            .file_selector_with_callback(move |playlist, items| {
                playlist.insert_playlist_items(0, &items);

                playlist.play_when_store_changed();
            });
    }

    #[template_callback]
    fn on_uri_activated(&self, _button: &gtk::Button) {
        let Some(playlist) = self.imp().playlist.upgrade() else {
            return;
        };

        playlist
            .imp()
            .uri_entry_with_callback(move |playlist, items| {
                playlist.insert_playlist_items(0, &vec![items]);

                playlist.play_when_store_changed();
            });
    }

    pub fn set_playlist(&self, playlist: Option<&PlayList>) {
        self.imp().playlist.set(playlist);
    }
}
