use anyhow::{format_err, Result};
use async_mutex::Mutex;
use enclose::enc;
use futures::{SinkExt, StreamExt};
use futures_channel::mpsc;
use log::*;
use std::default::Default;
use std::sync::Arc;
use uuid::Uuid;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;

use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtcp;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::rtp_transceiver::rtp_sender::RTCRtpSender;
use webrtc::track::track_remote::TrackRemote;

use crate::sfu::mediaengine;
use crate::sfu::routing::*;
use crate::sfu::session::{self, SessionEvent};
use crate::signal::signal;

// Peer ID unique to the connection/websocket
pub type Id = Uuid;

const TRANSPORT_TARGET_PUB: u32 = 0;
const TRANSPORT_TARGET_SUB: u32 = 1;

pub(super) type RtcpWriter = mpsc::Sender<Box<dyn rtcp::packet::Packet + Send + Sync>>;
pub(super) type RtcpReader = mpsc::Receiver<Box<dyn rtcp::packet::Packet + Send + Sync>>;

pub struct PeerConfig {
    pub setting_engine: SettingEngine,
    pub rtc_config: RTCConfiguration,
}

impl Default for PeerConfig {
    fn default() -> PeerConfig {
        PeerConfig {
            setting_engine: SettingEngine::default(),
            rtc_config: RTCConfiguration {
                ice_servers: vec![RTCIceServer {
                    urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                    ..Default::default()
                }],
                ..Default::default()
            },
        }
    }
}

/// Peer represents a WebRTC connection and it's tracks
/// We hold two publisher, subscriber
pub struct Peer {
    pub id: Id,
    pub publisher: Arc<RTCPeerConnection>,
    pub pub_rtcp_writer: RtcpWriter,

    pub subscriber: Arc<RTCPeerConnection>,
    pub sub_rtcp_writer: RtcpWriter,

    pub sub_pending_candidates: Arc<Mutex<Vec<RTCIceCandidateInit>>>,

    pub signal_tx: signal::WriteStream,
}

impl Peer {
    /// Creates a new Peer (with 2 peer connections)
    pub async fn new(
        signal_tx: signal::WriteStream,
        session_tx: mpsc::Sender<SessionEvent>,
        cfg: PeerConfig,
    ) -> Result<Arc<Peer>> {
        let (publisher, pub_rtcp_writer) = build_peer_connection(&cfg).await?;
        let (subscriber, sub_rtcp_writer) = build_peer_connection(&cfg).await?;

        let mut peer = Peer {
            id: Uuid::new_v4(),
            publisher,
            pub_rtcp_writer,
            subscriber,
            sub_rtcp_writer,
            sub_pending_candidates: Arc::new(Mutex::new(vec![])),
            signal_tx: signal_tx.clone(),
        };

        peer.setup_signal_hooks(signal_tx, session_tx).await;

        Ok(Arc::new(peer))
    }

    pub async fn publisher_get_answer_for_offer(
        &self,
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

    pub async fn subscriber_create_offer(&self) -> Result<RTCSessionDescription> {
        let offer = self.subscriber.create_offer(None).await?;

        let mut offer_gathering_complete = self.subscriber.gathering_complete_promise().await;
        self.subscriber.set_local_description(offer).await?;
        let _ = offer_gathering_complete.recv().await;

        let offer = self.subscriber.local_description().await.unwrap();
        Ok(offer)
    }

    pub async fn subscriber_set_answer(&self, answer: RTCSessionDescription) -> Result<()> {
        self.subscriber.set_remote_description(answer).await?;

        let mut pending_candidates = self.sub_pending_candidates.lock().await;
        while let Some(candidate) = (*pending_candidates).pop() {
            if let Err(err) = self.subscriber.add_ice_candidate(candidate).await {
                error!("error adding ice candidate: {}", err);
            }
        }

        Ok(())
    }

    // Adds a MediaTrackSubscriber to this peer's subscriber peer_connection
    pub async fn add_media_track_subscriber(&self, mut subscriber: MediaTrackSubscriber) {
        let rtp_sender: Arc<RTCRtpSender> = subscriber
            .add_to_peer_connection(&self.subscriber)
            .await
            .expect("error adding track subscriber to peer_connection");

        let sub_pc = Arc::downgrade(&self.subscriber);
        tokio::spawn(async move {
            subscriber.rtp_event_loop().await;

            if let Some(sub_pc) = sub_pc.upgrade() {
                if sub_pc.connection_state() == RTCPeerConnectionState::Closed {
                    return;
                }
                sub_pc
                    .remove_track(&rtp_sender)
                    .await
                    .expect("error removing track from subscriber");

                debug!("Removed track from subscriber");
            }
        });
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
                    let mut pending_candidates = self.sub_pending_candidates.lock().await;
                    debug!("subscriber pending candidate added");
                    pending_candidates.push(candidate);
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

    /// Closes both the publisher and subscriber peer connection
    pub async fn close(&self) {
        self.publisher
            .close()
            .await
            .expect("error closing publisher");
        self.subscriber
            .close()
            .await
            .expect("error closing subscriber");
    }

    /// This installs callbacks onto the peer connections that will send  
    /// signal::Event's back to the controlling websocket
    /// session::SessionEvent's to the controlling session
    pub async fn setup_signal_hooks(
        &mut self,
        sig_tx: signal::WriteStream,
        session_tx: session::WriteStream,
    ) {
        self.publisher
            .on_ice_candidate(Box::new(enc!( (sig_tx) move |c: Option<RTCIceCandidate>| {
                Box::pin(enc!( (sig_tx) async move {
                    if let Some(c) = c {
                        info!("on ice candidate publisher: {}", c);
                        sig_tx.unbounded_send(Ok(signal::Event::TrickleIce(signal::TrickleNotification {
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

        let pub_rtcp_tx = self.pub_rtcp_writer.clone();
        self.publisher
            .on_track(Box::new(enc!( (session_tx) {
                move |track: Option<Arc<TrackRemote>>, receiver: Option<Arc<RTCRtpReceiver>>| {
                    Box::pin( enc!( (mut session_tx, pub_rtcp_tx) async move {

                    if let (Some(track), Some(receiver)) = (track,receiver) {
                        tokio::spawn(async move {
                            let id = track.id().await;
                            let (media_track_router, closed) = MediaTrackRouter::new(track, receiver, pub_rtcp_tx).await;
                            session_tx.send(SessionEvent::TrackPublished(media_track_router.clone())).await.expect("error sending track router to session");
                            let _ = closed.await;
                            session_tx.send(SessionEvent::TrackRemoved(id)).await.expect("error sending track removed");
                        });
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
            .on_ice_candidate(Box::new(enc!( (sig_tx) move |c: Option<RTCIceCandidate>| {
                Box::pin(enc!( (sig_tx) async move {
                    if let Some(c) = c {
                        info!("on ice candidate subscriber: {}", c);
                        sig_tx.unbounded_send(Ok(signal::Event::TrickleIce(signal::TrickleNotification {
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

        let sub_pc = Arc::downgrade(&self.subscriber);
        self.subscriber
            .on_negotiation_needed(Box::new(enc!( (sig_tx, sub_pc) move || {
                Box::pin(enc!( (sig_tx, sub_pc) async move {
                    info!("subscriber on_negotiation_needed");

                    if let Some(sub_pc) = sub_pc.upgrade() {
                        if sub_pc.connection_state() == RTCPeerConnectionState::Closed {
                                return;
                        }

                        let offer = sub_pc.create_offer(None).await.expect("could not create subscriber offer:");
                        sub_pc.set_local_description(offer).await.expect("could not set local description");
                        let offer = sub_pc.local_description().await.unwrap();

                        info!("subscriber sending offer");
                        if let Err(_) = sig_tx.unbounded_send(Ok(signal::Event::SubscriberOffer(offer))) {
                            error!("signal connection closed");
                        }
                    }
                }))
            })))
            .await;
    }
}

/// Helper to build peer connections with the appropriate configuration
async fn build_peer_connection(cfg: &PeerConfig) -> Result<(Arc<RTCPeerConnection>, RtcpWriter)> {
    // Create a MediaEngine object to configure the supported codec
    let mut m = MediaEngine::default();
    mediaengine::register_default_codecs(&mut m)?;

    #[cfg(feature = "simulcast")]
    mediaengine::register_rtp_extension_simulcast(&mut m)?;

    #[cfg(feature = "audiolevel")]
    mediaengine::register_rtp_extension_audiolevel(&mut m)?;

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
        .with_setting_engine(cfg.setting_engine.clone())
        .build();

    trace!("building peer connection");

    // Create a new RTCPeerConnection
    let peer_connection = Arc::new(api.new_peer_connection(cfg.rtc_config.clone()).await?);

    let (rtcp_tx, mut rtcp_rx): (RtcpWriter, RtcpReader) = mpsc::channel(32);

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
