use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

mod imp {

    use super::*;

    #[derive(Debug, Default)]
    pub struct FughettaApplication;

    #[glib::object_subclass]
    impl ObjectSubclass for FughettaApplication {
        const NAME: &'static str = "FughettaApplication";
        type Type = super::FughettaApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for FughettaApplication {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_application_id(Some(crate::APP_ID));
            obj.set_resource_base_path(Some(crate::APP_RESOURCE_PATH));

            obj.set_accels_for_action("win.about", &["<Ctrl>N"]);
        }
    }

    impl ApplicationImpl for FughettaApplication {
        fn startup(&self) {
            self.parent_startup();

            mutsumi::init();
            crate::css::init();
            crate::FughettaWindow::ensure_type();
        }

        fn activate(&self) {
            self.parent_activate();

            let window = crate::FughettaWindow::new(&self.obj());
            // window.load_window_state();
            window.present();
        }
    }

    impl GtkApplicationImpl for FughettaApplication {}

    impl AdwApplicationImpl for FughettaApplication {}

    impl FughettaApplication {}
}

glib::wrapper! {
    pub struct FughettaApplication(ObjectSubclass<imp::FughettaApplication>)
        @extends gtk::gio::Application, gtk::Application, adw::Application, @implements gtk::gio::ActionGroup, gtk::gio::ActionMap;
}

impl Default for FughettaApplication {
    fn default() -> Self {
        Self::new()
    }
}

impl FughettaApplication {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
