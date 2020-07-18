extern crate gstreamer as gst;
extern crate gstreamer_video as gst_video;
use glib::prelude::*;

use std::sync::Arc;

use log::*;
use protoo_rs::server::WebsocketServer;

pub mod room;



fn main() {
    pretty_env_logger::init();
    log::set_max_level(LevelFilter::Trace);
    gst::init().unwrap();

    let ws = WebsocketServer{addr: "127.0.0.1:4242"};

    ws.run();
}
