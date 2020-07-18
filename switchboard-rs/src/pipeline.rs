use gtk::prelude::*;
use gdk::prelude::*;
use gst::prelude::*;
use gst_video::prelude::*;

use chrono::prelude::*;
use log::*;

use enclose::enc;
use failure::{Error, format_err};

use std::sync::{Arc, Mutex};
use std::sync::atomic::{
    AtomicBool,
    Ordering,
};


pub struct Room{
    pub pipeline: gst::Pipeline,
    pub peers: Vec<PeerConnection>,
}

pub struct PeerConnection {
    pub webrtcbin: gst::Element,
    pub audio_queue: gst::Element,
    pub video_queue: gst::Element,
}

