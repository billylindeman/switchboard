use gst::prelude::*;
use gst_video::prelude::*;

use std::result::Result;
use failure::{Error, format_err};

use log::*;


pub enum PeerMsg {
    OnIceCandidateCreated { candidate: String },
    AddIceCandidate{ candidate: String },

    SetRemoteDescription{ offer: String },
    OnSDPOfferCreated{ offer: String },
}

pub struct Peer {
    pub id: i64,
    pub pipeline: gst::Pipeline,
    pub webrtcbin: gst::Element,
}

impl Peer {
    pub fn new(peer_id: i64) -> Result<Peer, Error> { 
        let pipeline = gst::Pipeline::new(Some(&format!("peer{}", peer_id)));

        let webrtcbin = gst::ElementFactory::make(
            "webrtcbin",
            Some(&format!("peer{}webrtcbin", peer_id))
        ).or(Err(format_err!("Error creating webrtcbin")))?;

        let bus = pipeline.get_bus().unwrap();
        bus.add_signal_watch();
        bus.connect_message(move |_, msg| {
            use gst::MessageView;

            match msg.view() {
                MessageView::StateChanged(s) => debug!("state changed: {} {:?}", s.get_src().unwrap().get_name(), s),
                MessageView::Eos(..) => (),
                MessageView::Warning(warn) =>{
                    warn!("{} {:?} ", warn.get_error(), warn.get_debug());
                },
                MessageView::Error(err) => {
                    error!("{} {:?} ", err.get_error(), err.get_debug());
                    panic!("Pipeline Broken");
                },
                MessageView::Info(info) => {
                    info!("{} {:?} ", info.get_error(), info.get_debug());
                },
                _ => (),
            }
        });

        Ok(Peer{
            id: peer_id,
            pipeline: pipeline,
            webrtcbin: webrtcbin,
        })
    }
}

