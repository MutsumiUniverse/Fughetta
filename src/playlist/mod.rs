mod item;
mod source_row;
mod view;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{CompositeTemplate, gdk, gio, glib};
use mutsumi::{MutsumiPlayer, PlaylistItem};

pub use item::*;
pub use source_row::SourceActionRow;
pub use view::PlaylistView;

mod imp {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::OnceLock;

    use glib::subclass::InitializingObject;

    use crate::playlist::item::PlaylistFileItem;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/mutsumiuniverse/fughetta/ui/playlist.ui")]
    #[properties(wrapper_type = super::PlayList)]
    pub struct PlayList {
        #[template_child]
        pub bottom_sheet: TemplateChild<adw::BottomSheet>,
        #[template_child]
        pub view: TemplateChild<super::PlaylistView>,
        #[template_child]
        pub empty_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub drag_revealer: TemplateChild<gtk::Revealer>,

        #[property(get, set = Self::set_store, nullable)]
        pub store: RefCell<Option<gio::ListStore>>,

        pub player: glib::WeakRef<MutsumiPlayer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlayList {
        const NAME: &'static str = "PlayList";
        type Type = super::PlayList;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            super::SourceActionRow::ensure_type();
            super::PlaylistView::ensure_type();
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlayList {
        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: OnceLock<Vec<glib::subclass::Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    glib::subclass::Signal::builder("position-activated")
                        .param_types([i64::static_type()])
                        .build(),
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();

            self.view.connect_position_activated(glib::clone!(
                #[weak(rename_to = obj)]
                self.obj(),
                move |_, pos| {
                    obj.emit_by_name::<()>("position-activated", &[&pos]);
                }
            ));

            self.setup_file_drop();
        }
    }

    impl PlayList {
        fn set_store(&self, store: Option<gio::ListStore>) {
            self.view.set_store(store.as_ref());

            if let Some(store) = &store {
                store.connect_items_changed(glib::clone!(
                    #[weak(rename_to = imp)]
                    self,
                    move |store, _, _, _| imp.update_empty(store.n_items() == 0)
                ));
                self.update_empty(store.n_items() == 0);
            } else {
                self.update_empty(true);
            }

            self.store.replace(store);
        }

        fn update_empty(&self, empty: bool) {
            self.empty_page.set_visible(empty);
        }

        fn setup_file_drop(&self) {
            let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);

            drop.connect_enter(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                #[upgrade_or]
                gdk::DragAction::empty(),
                move |_, _, _| {
                    imp.drag_revealer.set_reveal_child(true);
                    gdk::DragAction::COPY
                }
            ));
            drop.connect_leave(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| imp.drag_revealer.set_reveal_child(false)
            ));
            drop.connect_drop(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                #[upgrade_or]
                false,
                move |_, value, _, _| {
                    imp.drag_revealer.set_reveal_child(false);
                    let Ok(files) = value.get::<gdk::FileList>() else {
                        return false;
                    };
                    let Some(store) = imp.store.borrow().clone() else {
                        return false;
                    };
                    let items: Vec<PlaylistItem> =
                        files.files().iter().map(PlaylistItem::from_file).collect();
                    if items.is_empty() {
                        return false;
                    }
                    store.extend_from_slice(&items);
                    true
                }
            ));

            self.obj().add_controller(drop);
        }
    }

    #[gtk::template_callbacks]
    impl PlayList {
        #[template_callback]
        fn on_open_sheet(&self) {
            self.bottom_sheet.set_open(true);
        }

        #[template_callback]
        fn revealer_visible(&self, reveal_child: bool, child_revealed: bool) -> bool {
            reveal_child || child_revealed
        }

        #[template_callback]
        pub fn on_file_selector_activated(&self) {
            self.bottom_sheet.set_open(false);

            self.file_selector_with_callback(move |imp, items| {
                let Some(store) = imp.store.borrow().clone() else {
                    return;
                };

                store.extend_from_slice(&items);
            });
        }

        #[template_callback]
        pub fn on_folder_selector_activated(&self) {
            self.bottom_sheet.set_open(false);

            self.folder_selector_with_callback(move |imp, items| {
                let Some(store) = imp.store.borrow().clone() else {
                    return;
                };

                store.extend_from_slice(&items);
            });
        }

        pub fn file_selector_with_callback(
            &self,
            callback: impl Fn(&Self, Vec<PlaylistItem>) + 'static,
        ) {
            let obj = self.obj();
            let dialog = gtk::FileDialog::builder()
                .title("Add Sources")
                .modal(true)
                .build();
            let root = obj.root().and_downcast::<gtk::Window>();

            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                async move {
                    let Ok(files) = dialog.open_multiple_future(root.as_ref()).await else {
                        return;
                    };

                    let files: Vec<gio::File> = files
                        .into_iter()
                        .flatten()
                        .filter_map(|file| file.downcast::<gio::File>().ok())
                        .collect();

                    if files.is_empty() {
                        return;
                    }

                    let items: Vec<PlaylistItem> =
                        files.iter().map(PlaylistItem::from_file).collect();

                    callback(&imp, items);
                }
            ));
        }

        pub fn folder_selector_with_callback(
            &self,
            callback: impl Fn(&Self, Vec<PlaylistItem>) + 'static,
        ) {
            let obj = self.obj();
            let dialog = gtk::FileDialog::builder()
                .title("Add Sources")
                .modal(true)
                .build();
            let root = obj.root().and_downcast::<gtk::Window>();

            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                async move {
                    let Ok(files) = dialog.select_multiple_folders_future(root.as_ref()).await
                    else {
                        return;
                    };

                    let files: Vec<gio::File> = files
                        .into_iter()
                        .flatten()
                        .filter_map(|file| file.downcast::<gio::File>().ok())
                        .collect();

                    if files.is_empty() {
                        return;
                    }

                    let items: Vec<PlaylistItem> =
                        files.iter().map(PlaylistItem::from_file).collect();

                    callback(&imp, items);
                }
            ));
        }

        #[template_callback]
        pub fn on_uri_activated(&self) {
            self.bottom_sheet.set_open(false);

            self.uri_entry_with_callback(move |imp, item| {
                let Some(store) = imp.store.borrow().clone() else {
                    return;
                };

                store.append(&item);
            });
        }

        pub fn uri_entry_with_callback(&self, callback: impl Fn(&Self, PlaylistItem) + 'static) {
            let obj = self.obj();
            let entry = adw::EntryRow::builder().title("Source Link").build();
            let list = gtk::ListBox::builder()
                .selection_mode(gtk::SelectionMode::None)
                .css_classes(["boxed-list"])
                .build();
            list.append(&entry);

            let dialog = adw::AlertDialog::new(Some("Add From URI"), None);
            dialog.set_width_request(500);
            dialog.set_extra_child(Some(&list));
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("add", "Add");
            dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("add"));
            dialog.set_close_response("cancel");

            glib::spawn_future_local(glib::clone!(
                #[weak]
                obj,
                #[weak]
                entry,
                async move {
                    let response = dialog.choose_future(Some(&obj)).await;

                    if response != "add" {
                        return;
                    }
                    let url = entry.text().to_string();

                    if url.is_empty() {
                        return;
                    }

                    let item = PlaylistItem::with_full_uri(&url);

                    callback(obj.imp(), item);
                }
            ));
        }

        pub fn append_playlist_items(&self, items: &[PlaylistItem]) {
            let Some(store) = self.store.borrow().clone() else {
                return;
            };

            store.extend_from_slice(items);
        }

        pub fn insert_playlist_items(&self, position: u32, items: &[PlaylistItem]) {
            let Some(store) = self.store.borrow().clone() else {
                return;
            };

            store.splice(position, 0, items);
        }

        pub fn play_when_store_changed(&self) {
            let Some(player) = self.player.upgrade() else {
                return;
            };

            let handler = Rc::new(RefCell::new(None));

            let id = player.connect_playlist_updated(glib::clone!(
                #[weak]
                player,
                #[strong]
                handler,
                move |_| {
                    player.set_playlist_pos(0);

                    if let Some(id) = handler.borrow_mut().take() {
                        player.disconnect(id)
                    }
                }
            ));

            *handler.borrow_mut() = Some(id);
        }

        #[template_callback]
        fn on_repeat_mode_notify(&self, _pspec: glib::ParamSpec, group: &adw::ToggleGroup) {
            let Some(p) = self.player.upgrade() else {
                return;
            };

            let Some(mode) = group.active_name() else {
                return;
            };

            let player = p.player();

            match mode.as_str() {
                "consecutive" => {
                    player.set_loop_playlist("no");
                    player.set_loop_file("no");
                }
                "repeat" => {
                    player.set_loop_playlist("yes");
                    player.set_loop_file("no");
                }
                "repeat-one" => {
                    player.set_loop_playlist("no");
                    player.set_loop_file("yes");
                }
                _ => (),
            }
        }

        #[template_callback]
        fn on_shuffle_notify(&self, _pspec: glib::ParamSpec, group: &adw::ToggleGroup) {
            let Some(p) = self.player.upgrade() else {
                return;
            };

            let Some(mode) = group.active_name() else {
                return;
            };

            let player = p.player();

            match mode.as_str() {
                "unshuffle" => {
                    player.playlist_unshuffle();
                }
                "shuffle" => {
                    player.playlist_shuffle();
                }
                _ => (),
            }
        }
    }

    impl WidgetImpl for PlayList {}
    impl BinImpl for PlayList {}
}

glib::wrapper! {
    pub struct PlayList(ObjectSubclass<imp::PlayList>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for PlayList {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayList {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn with_store(store: &gio::ListStore) -> Self {
        glib::Object::builder().property("store", store).build()
    }

    pub fn connect_position_activated<F: Fn(&Self, i64) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_closure(
            "position-activated",
            false,
            glib::closure_local!(move |obj: Self, pos: i64| {
                f(&obj, pos);
            }),
        )
    }

    pub fn set_player(&self, player: Option<&MutsumiPlayer>) {
        self.imp().player.set(player);

        self.imp().view.set_player(player);
    }
}
