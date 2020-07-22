use gst::prelude::*;
use gst_video::prelude::*;

use uuid::Uuid;

use std::result::Result;
use failure::{Error, format_err};
use async_std::sync::channel;
use async_std::task;

use log::*;

const STUN_SERVER: &str = "stun://stun.l.google.com:19302";

pub enum PeerMsg {
    AddIceCandidate{ sdp_mline_index: u32, candidate: String },
    OnIceCandidateCreated { candidate: String },

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

        let (tx, rx) = channel::<PeerMsg>(1);

        webrtcbin.set_property_from_str("stun-server", STUN_SERVER);
        webrtcbin.set_property_from_str("bundle-policy", "max-bundle");


        pipeline.add_many(&[
            &webrtcbin
        ])?;

        Ok(PeerConnection{
            id: peer_id,
            webrtcbin: webrtcbin,
        })
    }
}

