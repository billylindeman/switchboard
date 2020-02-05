use gtk::prelude::*;

use std::{
    result::Result,
};
use failure::{Error, format_err};

pub fn new() -> Result<gtk::Window, Error> {

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("TapeDeck");
    window.set_default_size(800,600);

    let stack = gtk::StackBuilder::new()
        .transition_type(gtk::StackTransitionType::OverRightLeft)
        .build();

    let sidebar = gtk::StackSidebarBuilder::new()
        .stack(&stack)
        .build();

    let panel = gtk::Paned::new(gtk::Orientation::Horizontal);
    panel.add(&sidebar);
    panel.add(&stack);

    window.add(&panel);


    let labels = [
        "Front Door",
        "Garage",
        "Patio",
        "Side Door"
    ];

    for name in &labels {
        let l = gtk::LabelBuilder::new()
            .label(name)
            .build();

        stack.add_titled(&l, name, name);
    }
        
    window.show_all();
    return Ok(window);
}
