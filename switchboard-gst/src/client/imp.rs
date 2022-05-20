use glib;
use gstreamer as gst;
use gstreamer_base as gst_base;

use gst_base::subclass::prelude::*;

use once_cell::sync::Lazy;
use std::i32;
use std::sync::Mutex;

#[derive(Default)]
pub struct Client {}

impl Client {}

#[glib::object_subclass]
impl ObjectSubclass for Client {
    const NAME: &'static str = "Client";
    type Type = super::Client;
    type ParentType = gst_base::BaseTransform;
}
