use anyhow::{format_err, Result};
use async_mutex::Mutex;
use enclose::enc;
use futures::StreamExt;
use futures_channel::mpsc;
use log::*;
use std::sync::Arc;
use uuid::Uuid;

use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtcp;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::track::track_remote::TrackRemote;

use crate::sfu::routing::*;
use crate::signal::signal;

// Peer ID unique to the connection/websocket
pub type Id = Uuid;

const TRANSPORT_TARGET_PUB: u32 = 0;
const TRANSPORT_TARGET_SUB: u32 = 1;

pub(super) type RtcpWriter = mpsc::Sender<Box<dyn rtcp::packet::Packet + Send + Sync>>;
pub(super) type RtcpReader = mpsc::Receiver<Box<dyn rtcp::packet::Packet + Send + Sync>>;

pub struct Peer {
    pub publisher: Arc<RTCPeerConnection>,
    pub pub_rtcp_writer: RtcpWriter,

    pub subscriber: Arc<RTCPeerConnection>,
    pub sub_rtcp_writer: RtcpWriter,

    pub sub_pending_candidates: Arc<Mutex<Vec<RTCIceCandidateInit>>>,
}

impl Peer {
    pub async fn new() -> Result<Peer> {
        let (publisher, pub_rtcp_writer) = build_peer_connection().await?;
        let (subscriber, sub_rtcp_writer) = build_peer_connection().await?;

        Ok(Peer {
            publisher,
            pub_rtcp_writer,
            subscriber,
            sub_rtcp_writer,
            sub_pending_candidates: Arc::new(Mutex::new(vec![])),
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

    pub async fn subscriber_create_offer(&mut self) -> Result<RTCSessionDescription> {
        let offer = self.subscriber.create_offer(None).await?;

        let mut offer_gathering_complete = self.subscriber.gathering_complete_promise().await;
        self.subscriber.set_local_description(offer).await?;
        let _ = offer_gathering_complete.recv().await;

        let offer = self.subscriber.local_description().await.unwrap();
        Ok(offer)
    }

    pub async fn subscriber_set_answer(&mut self, answer: RTCSessionDescription) -> Result<()> {
        self.subscriber.set_remote_description(answer).await?;

        if let mut pending_candidates = self.sub_pending_candidates.lock().await {
            while let Some(candidate) = (*pending_candidates).pop() {
                if let Err(err) = self.subscriber.add_ice_candidate(candidate).await {
                    error!("error adding ice candidate: {}", err);
                }
            }
        }

        Ok(())
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
            TRANSPORT_TARGET_SUB => match self.subscriber.remote_description().await {
                None => {
                    if let mut pending_candidates = self.sub_pending_candidates.lock().await {
                        debug!("subscriber pending candidate added");
                        pending_candidates.push(candidate);
                    }
                }
                Some(_) => {
                    if let Err(err) = self.subscriber.add_ice_candidate(candidate).await {
                        error!("error adding ice candidate: {}", err);
                    }
                }
            },

            _ => {}
        }
        Ok(())
    }

    pub async fn close(&self) {
        self.publisher.close().await.unwrap();
    }

    pub async fn event_loop(&mut self, mut rx: signal::ReadStream, tx: signal::WriteStream) {
        self.publisher
            .on_ice_candidate(Box::new(enc!( (tx) move |c: Option<RTCIceCandidate>| {
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
            })))
            .await;

        //@test loopback tracks atm
        let pub_pc = Arc::downgrade(&self.publisher);
        let sub_pc = Arc::downgrade(&self.subscriber);

        let pub_rtcp_tx = self.pub_rtcp_writer.clone();
        self.publisher
            .on_track(Box::new(enc!( (pub_pc, sub_pc) {
                move |track: Option<Arc<TrackRemote>>, receiver: Option<Arc<RTCRtpReceiver>>| {
                    Box::pin( enc!( (pub_pc, sub_pc, pub_rtcp_tx) async move {

                    if let (Some(track), Some(receiver)) = (track,receiver) {
                        let mut media_track_router = MediaTrackRouter::new(track, receiver, pub_rtcp_tx).await;
                        media_track_router.event_loop().await;

                        if let Some(sub_pc) = sub_pc.upgrade() {
                            let mut media_track_subscriber = media_track_router.add_subscriber().await;
                            media_track_subscriber.add_to_peer_connection(&sub_pc).await.expect("error adding track subscriber to peer_connection");
                            tokio::spawn(async move {
                                media_track_subscriber.event_loop().await;
                            });
                        }

                    } else {
                        warn!("on track called with no track!");
                    }
                }))
                }}
            )))
            .await;

        let _ = self
            .subscriber
            .create_data_channel("switchboard-rx", None)
            .await;

        self.subscriber
            .on_ice_candidate(Box::new(enc!( (tx) move |c: Option<RTCIceCandidate>| {
                Box::pin(enc!( (tx) async move {
                    if let Some(c) = c {
                        info!("on ice candidate subscriber: {}", c);
                        tx.unbounded_send(Ok(signal::Event::TrickleIce(signal::TrickleNotification {
                            target: TRANSPORT_TARGET_SUB,
                            candidate: c
                                .to_json()
                                .await
                                .expect("error converting to json")
                                .into(),
                        }))).expect("error sending ice");
                    }
                }))
            })))
            .await;

        self.subscriber
            .on_negotiation_needed(Box::new(enc!( (tx, sub_pc) move || {
                Box::pin(enc!( (tx, sub_pc) async move {
                    info!("subscriber on_negotiation_needed");

                    if let Some(sub_pc) = sub_pc.upgrade() {
                        let offer = sub_pc.create_offer(None).await.unwrap();
                        sub_pc.set_local_description(offer).await.unwrap();
                        let offer = sub_pc.local_description().await.unwrap();

                        info!("subscriber sending offer");
                        tx.unbounded_send(Ok(signal::Event::SubscriberOffer(offer)))
                            .expect("error sending subscriber offer");
                    }
                }))
            })))
            .await;

        //if let Ok(offer) = self.subscriber_create_offer().await {
        //    info!("sending subscriber offer");
        //    tx.unbounded_send(Ok(signal::Event::SubscriberOffer(offer)))
        //        .expect("error sending subscriber offer");
        //}

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

                signal::Event::SubscriberAnswer(answer) => {
                    info!("subscriber got answer");
                    self.subscriber_set_answer(answer.desc)
                        .await
                        .expect("subscriber error setting remote description");
                }
                _ => {}
            }
        }

        info!("event loop finished")
    }
}

async fn build_peer_connection() -> Result<(Arc<RTCPeerConnection>, RtcpWriter)> {
    // Create a MediaEngine object to configure the supported codec
    let mut m = MediaEngine::default();
    m.register_default_codecs()?;

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
    let peer_connection = Arc::new(api.new_peer_connection(config).await?);

    let (rtcp_tx, mut rtcp_rx) = mpsc::channel(32);

    // Spawn RTCP Write Loop
    tokio::spawn(enc!( (peer_connection) async move {
        debug!("Peer RTCP WriteLoop: started");

        while let Some(rtcp) = rtcp_rx.next().await {
            if let Err(e) = peer_connection.write_rtcp(&[rtcp]).await {
                error!("Peer RTCP WriteLoop: error writing rtcp {} ", e);
            }
        }

        debug!("Peer RTCP WriteLoop: ended");
    }));

    Ok((peer_connection, rtcp_tx))
}
