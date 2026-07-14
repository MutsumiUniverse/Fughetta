use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

mod imp {
    use std::ops::ControlFlow;

    use crate::{FughettaWindow, args::Args};

    use super::*;

    #[derive(Debug, Default)]
    pub struct FughettaApplication;

    #[glib::object_subclass]
    impl ObjectSubclass for FughettaApplication {
        const NAME: &'static str = "FughettaApplication";
        type Type = super::FughettaApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for FughettaApplication {}

    impl ApplicationImpl for FughettaApplication {
        fn startup(&self) {
            self.parent_startup();

            crate::css::init();
            mutsumi::init();

            crate::FughettaWindow::ensure_type();

            let window = crate::FughettaWindow::new(&self.obj());
            // window.load_window_state();
            window.present();
        }

        fn handle_local_options(&self, options: &glib::VariantDict) -> ControlFlow<glib::ExitCode> {
            let Ok(log_level) = options.lookup::<String>("log-level") else {
                unreachable!()
            };

            Args { log_level }.init();

            ControlFlow::Continue(())
        }

        fn open(&self, files: &[gtk::gio::File], _hint: &str) {
            let Some(active_window) = self.obj().active_window() else {
                return;
            };

            let Some(window) = active_window.downcast_ref::<FughettaWindow>() else {
                return;
            };

            window.open_files(files);

            window.present();
        }
    }

    impl GtkApplicationImpl for FughettaApplication {}

    impl AdwApplicationImpl for FughettaApplication {}

    impl FughettaApplication {}
}

glib::wrapper! {
    pub struct FughettaApplication(ObjectSubclass<imp::FughettaApplication>)
        @extends gtk::gio::Application, gtk::Application, adw::Application, @implements gtk::gio::ActionGroup, gtk::gio::ActionMap, gtk::Buildable;
}

impl Default for FughettaApplication {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl FughettaApplication {
    pub fn new() -> Self {
        let app: Self = glib::Object::builder()
            .property("application-id", crate::APP_ID)
            .property("flags", gtk::gio::ApplicationFlags::HANDLES_OPEN)
            .build();

        app.set_resource_base_path(Some(crate::APP_RESOURCE_PATH));

        app.add_main_option(
            "log-level",
            b'l'.into(),
            glib::OptionFlags::NONE,
            glib::OptionArg::String,
            "Logging level",
            Some("LEVEL"),
        );

        app
    }
}
