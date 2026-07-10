use std::{env, io, path::PathBuf};

use clap::Parser;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt::time::ChronoLocal};

use crate::{ARG_FILES, CLIENT_VERSION, dyn_event};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    pub file: Vec<PathBuf>,

    #[clap(long, short)]
    log_level: Option<String>,
}

impl Args {
    /// Build the tracing subscriber using parameters from the command line
    /// arguments
    ///
    /// ## Panics
    ///
    /// Panics if the log file cannot be opened.
    fn init_tracing_subscriber(&self) {
        let level = match self.log_level.as_deref() {
            Some(level) if ["error", "warn", "info", "debug", "trace"].contains(&level) => level,
            _ => "info",
        };
        let filter = EnvFilter::builder().parse_lossy(format!("{level},glycin=error"));
        let builder = tracing_subscriber::fmt()
            .with_timer(ChronoLocal::rfc_3339())
            .with_env_filter(filter);

        builder.with_writer(io::stderr).init()
    }

    fn init_glib_to_tracing(&self) {
        gtk::glib::log_set_writer_func(|level, x| {
            let domain = x
                .iter()
                .find(|&it| it.key() == "GLIB_DOMAIN")
                .and_then(|it| it.value_str());
            let Some(message) = x
                .iter()
                .find(|&it| it.key() == "MESSAGE")
                .and_then(|it| it.value_str())
            else {
                return gtk::glib::LogWriterOutput::Unhandled;
            };

            match domain {
                Some(domain) => {
                    dyn_event!(level, domain = %domain, message);
                }
                None => {
                    dyn_event!(level, message);
                }
            }
            gtk::glib::LogWriterOutput::Handled
        });

        info!("Glib logging redirected to tracing");
    }

    pub fn init_files(&self) {
        let files = self
            .file
            .iter()
            .map(gtk::gio::File::for_path)
            .collect();

        ARG_FILES.set(files).expect("Failed to set ARG_FILES???");
    }

    pub fn init(&self) {
        self.init_tracing_subscriber();
        self.init_glib_to_tracing();
        self.init_files();

        std::panic::set_hook(Box::new(|info| {
            if let Some(s) = info.payload().downcast_ref::<&str>() {
                eprintln!("{s}");
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                eprintln!("{s}");
            }
            if let Some(loc) = info.location() {
                eprintln!("At {}:{}", loc.file(), loc.line());
            }
        }));

        info!("Args: {:?}", self);

        info!(
            "Application Version: {}, Platform: {} {}, CPU Architecture: {}",
            CLIENT_VERSION,
            env::consts::OS,
            env::consts::FAMILY,
            env::consts::ARCH
        );
    }
}
