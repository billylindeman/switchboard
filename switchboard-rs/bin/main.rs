use log::*;

use switchboard::signal;

fn main() {
    pretty_env_logger::init();
    log::set_max_level(LevelFilter::Trace);
    gst::init().unwrap();

    signal::Server::run();
}
