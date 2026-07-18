mod about;

use std::time::Duration;

use about::FughettaAboutDialog;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{
    CompositeTemplate,
    glib::{self, spawn_future_local},
};

use mutsumi::{MutsumiPlayer, PlaylistItem};

use crate::playlist::{PlayList, PlaylistFileItem};

mod imp {

    use crate::playlist::PlaylistFileItem;
    use glib::subclass::InitializingObject;
    use gtk::{gdk, glib::WeakRef};
    use mutsumi::PlaylistItem;

    use crate::status::PlaceHolderStatus;

    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/io/github/mutsumiuniverse/fughetta/ui/window.ui")]
    pub struct FughettaWindow {
        #[template_child]
        pub player: TemplateChild<MutsumiPlayer>,

        pub playlist: WeakRef<PlayList>,
        pub about_dialog: FughettaAboutDialog,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FughettaWindow {
        const NAME: &'static str = "FughettaWindow";
        type Type = super::FughettaWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            MutsumiPlayer::ensure_type();
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FughettaWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.bind_action_entries();

            let playlist = PlayList::with_store(&self.player.playlist_store());

            self.playlist.set(Some(&playlist));
            playlist.set_player(Some(&self.player));
            playlist.connect_position_activated(glib::clone!(
                #[weak(rename_to = window)]
                obj,
                move |_, pos| {
                    window.imp().player.set_playlist_pos(pos);
                }
            ));

            self.player.playlist_bin().set_child(Some(&playlist));
            self.player.playlist_stack_page().set_visible(true);

            let place_holder_status = PlaceHolderStatus::new();
            place_holder_status.set_playlist(Some(&playlist));

            self.player
                .overlay_status()
                .set_child(Some(&place_holder_status));

            self.player.set_about_handler_with_label(
                "About Fughetta",
                glib::clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.present_about_dialog();
                    }
                ),
            );

            self.setup_drag_bin();
            self.setup_file_drop();
        }
    }

    impl WidgetImpl for FughettaWindow {}
    impl WindowImpl for FughettaWindow {}
    impl ApplicationWindowImpl for FughettaWindow {}
    impl AdwApplicationWindowImpl for FughettaWindow {}

    impl FughettaWindow {
        fn setup_drag_bin(&self) {
            let drag_bin = adw::Bin::builder().css_classes(vec!["drop-target"]).build();

            self.player.drag_revealer().set_child(Some(&drag_bin));
        }

        fn setup_file_drop(&self) {
            let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);

            drop.set_types(&[gdk::FileList::static_type(), String::static_type()]);

            let revealer = self.player.drag_revealer();

            drop.connect_enter(glib::clone!(
                #[weak]
                revealer,
                #[upgrade_or]
                gdk::DragAction::empty(),
                move |_, _, _| {
                    revealer.set_reveal_child(true);
                    gdk::DragAction::COPY
                }
            ));

            drop.connect_leave(glib::clone!(
                #[weak]
                revealer,
                move |_| {
                    revealer.set_reveal_child(false);
                }
            ));

            drop.connect_drop(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                #[upgrade_or]
                false,
                move |_, value, _, _| {
                    let items: Vec<PlaylistItem> = if let Ok(files) = value.get::<gdk::FileList>() {
                        files.files().iter().map(PlaylistItem::from_file).collect()
                    } else if let Ok(uri_list) = value.get::<String>() {
                        uri_list
                            .lines()
                            .map(str::trim)
                            .filter(|uri| !uri.is_empty() && !uri.starts_with('#'))
                            .map(|uri| PlaylistItem::with_full_uri(uri))
                            .collect()
                    } else {
                        return false;
                    };

                    if items.is_empty() {
                        return false;
                    }

                    let Some(playlist) = imp.playlist.upgrade() else {
                        return false;
                    };

                    playlist.imp().insert_playlist_items(0, &items);
                    playlist.imp().play_when_store_changed();

                    true
                }
            ));

            self.player.content_overlay().add_controller(drop);
        }
    }
}

glib::wrapper! {
    pub struct FughettaWindow(ObjectSubclass<imp::FughettaWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
                    gtk::Native, gtk::Root, gtk::ShortcutManager,
                    gtk::gio::ActionGroup, gtk::gio::ActionMap;
}

impl FughettaWindow {
    pub fn new(app: &crate::Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    pub fn open_files(&self, files: &[gtk::gio::File]) {
        if files.is_empty() {
            return;
        }

        let items: Vec<PlaylistItem> = files.iter().map(PlaylistItem::from_file).collect();

        let Some(playlist) = self.imp().playlist.upgrade() else {
            return;
        };

        // we need to wait mpv proxy and event loop init
        // and I have no idea how to do it properly, so just wait 200ms
        spawn_future_local(async move {
            let _ = glib::future_with_timeout(Duration::from_millis(200), async move {
                playlist.imp().insert_playlist_items(0, &items);
                playlist.imp().play_when_store_changed();
            })
            .await;
        });
    }

    pub fn bind_action_entries(&self) {
        let about_action = gtk::gio::ActionEntry::builder("about")
            .activate(|window: &FughettaWindow, _, _| {
                window.present_about_dialog();
            })
            .build();

        self.add_action_entries([about_action]);
    }

    pub fn present_about_dialog(&self) {
        // self.imp().about_dialog.present(None::<&gtk::Widget>);
        self.imp().about_dialog.present(Some(self));
    }
}
