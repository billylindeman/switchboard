use gst::prelude::*;
use gst::gst_element_error;
use gst_webrtc::prelude::*;

use uuid::Uuid;


use enclose::enc;

use std::result::Result;
use failure::{Error, format_err};
use crossbeam_channel::{bounded, Sender};

use crate::signal;

use log::*;

const STUN_SERVER: &str = "stun://stun.l.google.com:19302";

pub enum PeerEvent {
    OnIceCandidateCreated { sdp_mline_index: u32, candidate: String },
    OnOfferCreated{ offer: String },
    OnAnswerCreated{ answer: String},

    OnStreamAdded{},
}

pub struct PeerConnection {
    pub id: Uuid,
    pub webrtcbin: gst::Element,
    tx: Sender<PeerEvent>, 
}

impl PeerConnection {
    pub fn new(pipeline: &gst::Pipeline, peer_id: Uuid) -> Result<PeerConnection, Error> { 

        let webrtcbin = gst::ElementFactory::make(
            "webrtcbin",
            Some(&format!("peer{}webrtcbin", peer_id))
        )?;

        let (tx, rx) = bounded::<PeerEvent>(1);

        webrtcbin.set_property_from_str("stun-server", STUN_SERVER);
        webrtcbin.set_property_from_str("bundle-policy", "max-bundle");


        webrtcbin
            .connect("on-negotiation-needed", false, enc!( (tx) move |values| {
                let _webrtc = values[0].get::<gst::Element>().unwrap();
                debug!("starting negotiation");

                let promise = gst::Promise::new_with_change_func(enc!( (_webrtc, tx) move |reply| {
                    let reply = match reply { 
                        Ok(reply) => reply,
                        Err(err) => return gst_element_error!(
                            _webrtc,
                            gst::LibraryError::Failed,
                            ("Failed to create offer: {:?}", err)
                        )
                    };

                    let offer = reply
                        .get_value("offer")
                        .unwrap()
                        .get::<gst_webrtc::WebRTCSessionDescription>()
                        .expect("Invalid argument")
                        .unwrap();

                    _webrtc 
                        .unwrap()
                        .emit("set-local-description", &[&offer, &None::<gst::Promise>])
                        .unwrap();
       
                    println!(
                       "sending SDP offer to peer: {}",
                       offer.get_sdp().as_text().unwrap()
                    );
 
                    let message = PeerEvent::OnOfferCreated{
                        offer: offer.get_sdp().as_text().unwrap(),
                    };

                    let res = tx.send(message);
                    match res {
                        Ok(_) => None,
                        Err(err) => gst_element_error!(
                            _webrtc,
                            gst::LibraryError::Failed,
                            ("Failed to send SDP Offer {:?}", err)
                        )
                    }
                }));

                webrtcbin
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

                let res = tx.send(PeerEvent::OnIceCandidateCreated{
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
            tx: tx,
        })
    }
    

    pub fn set_remote_description(&self, sdp: signal::SDP) -> Result<(), Error> {
        use signal::SDP;

        let (sdp_type, sdp_msg) = match sdp {
            SDP::Offer{sdp} => {(
                gst_webrtc::WebRTCSDPType::Offer,
                gst_sdp::SDPMessage::parse_buffer(sdp.as_bytes())
                    .or(Err(format_err!("Failed to parse SDP offer")))?
            )}
            SDP::Answer{sdp} => {(
                gst_webrtc::WebRTCSDPType::Answer,
                gst_sdp::SDPMessage::parse_buffer(sdp.as_bytes())
                    .or(Err(format_err!("Failed to parse SDP answer")))?
            )}
        };

        let gst_sdp = gst_webrtc::WebRTCSessionDescription::new(sdp_type, sdp_msg);

        self.webrtcbin
            .emit("set-remote-description", &[&gst_sdp, &None::<gst::Promise>])
            .unwrap();

        Ok(())
    }


    pub fn create_answer(&self) -> Result<(), Error> {
        let tx = self.tx.clone();
        let _webrtc = self.webrtcbin.clone();

        let promise = gst::Promise::new_with_change_func(move |reply| {
            let reply = match reply { 
                Ok(reply) => reply,
                Err(err) => return gst_element_error!(
                    _webrtc,
                    gst::LibraryError::Failed,
                    ("Failed to create answer: {:?}", err)
                )
            };

            let answer = reply
                .get_value("answer")
                .unwrap()
                .get::<gst_webrtc::WebRTCSessionDescription>()
                .expect("Invalid argument")
                .unwrap();

            self.webrtcbin
                .emit("set-local-description", &[&answer, &None::<gst::Promise>])
                .unwrap();
        
            let message = PeerEvent::OnAnswerCreated {
                answer: answer.get_sdp().as_text().unwrap(),
            };

            let res = tx.send(message);
            match res {
                Ok(_) => None,
                Err(err) => gst_element_error!(
                    _webrtc,
                    gst::LibraryError::Failed,
                    ("Failed to send answer: {:?}", err)
                )
            }
        });

        self.webrtcbin
            .emit("create-answer", &[&None::<gst::Structure>, &promise])?;

        Ok(())
    }

    pub fn add_ice_candidate(&self, sdp_mline_index: u32, candidate: String) -> Result<(), Error> {
        self.webrtcbin
            .emit("add-ice-candidate", &[&sdp_mline_index, &candidate])?;
        Ok(())
    }


}

