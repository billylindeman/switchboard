use gtk::prelude::*;

use std::{
    result::Result,
};
use failure::{Error, format_err};


fn build_header(name: &str) -> (gtk::Box, gtk::Button) {
    let label = gtk::Label::new(Some(name));

    let button = gtk::Button::new_from_icon_name(
        Some("list-add"), 
        gtk::IconSize::SmallToolbar
    );

    let container = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    container.pack_start(&label, true, true, 4);
    container.pack_end(&button, false, false, 4);

    (container, button)
}

pub fn new() -> Result<gtk::Window, Error> {

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("TapeDeck");
    window.set_default_size(800,600);

    let stack = gtk::StackBuilder::new()
        .transition_type(gtk::StackTransitionType::OverRightLeft)
        .build();


    // Setup Sidebar with two panels
    let sidebar = gtk::Paned::new(gtk::Orientation::Vertical);

    // Scenes Panel
    let (scenes_header, scenes_add_button) = build_header("Scenes");
    let scenes_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    scenes_box.set_size_request(96,240);
    let scenes_list = gtk::ListBox::new();
    scenes_box.pack_start(&scenes_header, false, false, 2);
    scenes_box.pack_end(&scenes_list, true, true, 2);


    // Cameras Panel
    let (cameras_header, cameras_add_button) = build_header("Camera");
    let cameras_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    cameras_box.set_size_request(96,240);
    let cameras_list = gtk::ListBox::new();
    cameras_box.pack_start(&cameras_header, false, false, 2);
    cameras_box.pack_end(&cameras_list, true, true, 2);


    scenes_add_button.connect_clicked(|_| {
        println!("Add Scene");
    });

    cameras_add_button.connect_clicked(|_| {
        println!("Add Camera!");
    });

    sidebar.add(&scenes_box);
    sidebar.add(&cameras_box);

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
