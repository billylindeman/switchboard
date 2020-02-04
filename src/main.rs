use gtk::prelude::*;

use std::result::Result;
use std::sync::Arc;

use log::*;
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

        ApplicationRef::new(a)
    }
}

fn main() {
    let app = Application::new();
}
