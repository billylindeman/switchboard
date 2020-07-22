use gst::prelude::*;
use gst::gst_element_error;

use gst_video::prelude::*;

use uuid::Uuid;


use enclose::enc;

use std::result::Result;
use failure::{Error, format_err};
use crossbeam_channel::bounded;

use log::*;

const STUN_SERVER: &str = "stun://stun.l.google.com:19302";

pub enum PeerMsg {
    AddIceCandidate{ sdp_mline_index: u32, candidate: String },
    OnIceCandidateCreated { sdp_mline_index: u32, candidate: String },

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

        let (tx, rx) = bounded::<PeerMsg>(1);

        webrtcbin.set_property_from_str("stun-server", STUN_SERVER);
        webrtcbin.set_property_from_str("bundle-policy", "max-bundle");


        webrtcbin
            .connect("on-negotiation-needed", false, enc!( (tx) move |values| {
                let _webrtc = values[0].get::<gst::Element>().unwrap();
                debug!("starting negotiation");

                let promise = gst::Promise::new_with_change_func(move |reply| {

                    if let Err(err) = peer.on_offer_created(reply) {
                        gst_element_error!(
                            peer.bin,
                            gst::LibraryError::Failed,
                            ("Failed to send SDP offer: {:?}", err)
                        );
                    }
                });

        self.webrtcbin
            .emit("create-offer", &[&None::<gst::Structure>, &promise])
            .unwrap();

 

                None
            }))
            .unwrap();


        // Connect crossbeam channel to webrtcbin hooks
        webrtcbin
            .connect("on-ice-candidate", false, enc!((tx) move |values| {
                let _webrtc = values[0].get::<gst::Element>().expect("Invalid argument");
                let mlineindex = values[1].get_some::<u32>().expect("Invalid argument");
                let candidate = values[2]
                    .get::<String>()
                    .expect("Invalid argument")
                    .unwrap();

                let res = tx.send(PeerMsg::OnIceCandidateCreated{
                    sdp_mline_index: mlineindex,
                    candidate,
                });

                match res {
                    Ok(_) => None,
                    Err(err) => gst_element_error!(
                        _webrtc,
                        gst::LibraryError::Failed,
                        ("Failed to send ICE candidate: {:?}", err)
                    )
                }
            }))
            .unwrap();


        pipeline.add_many(&[
            &webrtcbin
        ])?;

        Ok(PeerConnection{
            id: peer_id,
            webrtcbin: webrtcbin,
        })
    }
}

