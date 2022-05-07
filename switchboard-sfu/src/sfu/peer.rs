use crate::signal::signal;
use anyhow::{format_err, Result};
use async_mutex::Mutex;
use enclose::enc;
use futures::{Stream, StreamExt};
use log::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264, MIME_TYPE_OPUS};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::media::io::h264_writer::H264Writer;
use webrtc::media::io::ogg_writer::OggWriter;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
};
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::track::track_remote::TrackRemote;

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionDescription {
    #[serde(rename = "type")]
    pub t: String,
    pub sdp: String,
}

const TRANSPORT_TARGET_PUB: u32 = 0;
const TRANSPORT_TARGET_SUB: u32 = 1;

pub struct Peer {
    pub publisher: RTCPeerConnection,
    //    pub subscriber: Arc<RTCPeerConnection>,
}

impl Peer {
    pub async fn new() -> Result<Peer> {
        Ok(Peer {
            publisher: build_peer_connection().await?,
            //            subscriber: build_peer_connection().await?,
        })
    }

    pub async fn publisher_get_answer_for_offer(
        &mut self,
        offer: RTCSessionDescription,
    ) -> Result<RTCSessionDescription> {
        debug!("publisher set remote description");
        self.publisher.set_remote_description(offer).await?;

        let answer = self.publisher.create_answer(None).await?;
        self.publisher.set_local_description(answer).await?;

        match self.publisher.local_description().await {
            Some(answer) => Ok(answer),
            None => Err(format_err!("couldn't set local description")),
        }
    }

    pub async fn trickle_ice_candidate(
        &self,
        target: u32,
        candidate: RTCIceCandidateInit,
    ) -> Result<()> {
        match target {
            TRANSPORT_TARGET_PUB => {
                if let Err(err) = self.publisher.add_ice_candidate(candidate).await {
                    error!("error adding ice candidate: {}", err);
                }
            }
            //           TRANSPORT_TARGET_SUB => self.subscriber.add_ice_candidate(candidate).await?,
            _ => {}
        }
        Ok(())
    }

    pub async fn close(&self) {
        self.publisher.close().await;
    }

    pub async fn event_loop(&mut self, mut rx: signal::ReadStream, tx: signal::WriteStream) {
        self.publisher
            .on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
                Box::pin(enc!( (tx) async move {
                    if let Some(c) = c {
                        info!("on ice candidate publisher: {}", c);
                        tx.unbounded_send(Ok(signal::Event::TrickleIce(signal::TrickleNotification {
                            target: TRANSPORT_TARGET_PUB,
                            candidate: c
                                .to_json()
                                .await
                                .expect("error converting to json")
                                .into(),
                        }))).expect("error sending ice");
                    }
                }))
            }))
            .await;

        while let Some(Ok(evt)) = rx.next().await {
            match evt {
                signal::Event::JoinRequest(res, join) => {
                    info!("got join request: {:#?}", join);

                    let answer = self.publisher_get_answer_for_offer(join.offer).await;
                    if let Err(err) = &answer {
                        error!("Error with join offer {}", err);
                    };

                    info!("answer created ");

                    res.send(answer.unwrap()).expect("error sending response");
                }

                signal::Event::TrickleIce(trickle) => {
                    info!("trickle ice: {:#?}", trickle);
                    self.trickle_ice_candidate(trickle.target, trickle.candidate.into())
                        .await
                        .expect("error adding trickle candidate");
                }

                signal::Event::PublisherOffer(res, offer) => {
                    info!("publisher made offer");

                    let answer = self
                        .publisher_get_answer_for_offer(offer.desc)
                        .await
                        .expect("publisher error setting remote description");

                    res.send(answer).expect("error sending answer");
                }
                _ => {}
            }
        }

        info!("event loop finished")
    }
}

async fn build_peer_connection() -> Result<RTCPeerConnection> {
    // Create a MediaEngine object to configure the supported codec
    let mut m = MediaEngine::default();
    m.register_default_codecs();

    // Create a InterceptorRegistry. This is the user configurable RTP/RTCP Pipeline.
    // This provides NACKs, RTCP Reports and other features. If you use `webrtc.NewPeerConnection`
    // this is enabled by default. If you are manually managing You MUST create a InterceptorRegistry
    // for each PeerConnection.
    let mut registry = Registry::new();

    // Use the default set of Interceptors
    registry = register_default_interceptors(registry, &mut m)?;

    // Create the API object with the MediaEngine
    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    // Prepare the configuration
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    trace!("building peer connection");

    // Create a new RTCPeerConnection
    let peer_connection = api.new_peer_connection(config).await?;

    Ok(peer_connection)
}