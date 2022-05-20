use glib;
use gstreamer as gst;
use gstreamer_base as gst_base;

use gst::prelude::*;

mod imp;

glib::wrapper! {
    pub struct Client(ObjectSubclass<imp::Client>) @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

// Registers the type for our element, and then registers in GStreamer under
// the name "rsrgb2gray" for being able to instantiate it via e.g.
// gst::ElementFactory::make().
pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "switchboardclient",
        gst::Rank::None,
        Client::static_type(),
    )
}
