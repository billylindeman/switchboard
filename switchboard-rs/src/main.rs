extern crate gstreamer as gst;
extern crate gstreamer_video as gst_video;
use glib::prelude::*;

use log::*;


pub mod room;
pub mod peer;
pub mod signal;
//pub mod sfu;

fn main() {
    pretty_env_logger::init();
    log::set_max_level(LevelFilter::Trace);
    gst::init().unwrap();

    signal::Server::run();
}
