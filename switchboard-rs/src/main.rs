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
