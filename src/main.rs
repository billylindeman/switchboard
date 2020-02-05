use std::result::Result;
use std::sync::Arc;
use log::*;


#[cfg(target_os="linux")]
use x11::xlib::XInitThreads;

use gtk::prelude::*;
use failure::{Error, format_err};

pub mod gui;

type ApplicationRef = Arc<Application>;
struct Application {
    pub window: gtk::Window,
}

impl Application {
    fn new() -> Result<ApplicationRef, Error> {
        let a = Application{
            window: gui::new()?,
        };
        a.window.connect_destroy(|_|{gtk::main_quit()});
        
        Ok(ApplicationRef::new(a))
    }

    fn run(&self) {
        gtk::main();
    }
}

fn main() {
    #[cfg(target_os="linux")]
    unsafe {
        // Initialize X11/XLib multhithreading
        // If this is not called, GTK/GLib will panic at some point
        XInitThreads();
    }

    pretty_env_logger::init();
    log::set_max_level(LevelFilter::Trace);

    gtk::init().expect("Error initializing gtk");

    let app = Application::new().expect("Error initializing application");
    app.run();
}
