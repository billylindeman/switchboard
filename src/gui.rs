use gtk::prelude::*;

use std::{
    result::Result,
};
use failure::{Error, format_err};

pub fn new() -> Result<gtk::Window, Error> {

    let w = gtk::Window::new(gtk::WindowType::Toplevel);
    w.set_title("TapeDeck");
    w.set_default_size(800,600);

    w.show();

    return Ok(w);
}