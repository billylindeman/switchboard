use gst::prelude::*;
use gst_video::prelude::*;

use uuid::Uuid;

use std::result::Result;
use failure::{Error, format_err};

use log::*;


pub enum PeerMsg {
    OnIceCandidateCreated { candidate: String },
    AddIceCandidate{ candidate: String },

    SetRemoteDescription{ offer: String },
    OnSDPOfferCreated{ offer: String },
}

pub struct PeerConnection {
    pub id: Uuid,
    pub webrtcbin: gst::Element,
}

impl PeerConnection {
    pub fn new(pipeline: &gst::Pipeline, peer_id: Uuid) -> Result<PeerConnection, Error> { 
        let webrtcbin = gst::ElementFactory::make(
            "webrtcbin",
            Some(&format!("peer{}webrtcbin", peer_id))
        )?;

        pipeline.add_many(&[
            &webrtcbin
        ])?;

        Ok(PeerConnection{
            id: peer_id,
            webrtcbin: webrtcbin,
        })
    }
}

