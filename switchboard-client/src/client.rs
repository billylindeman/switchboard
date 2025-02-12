use anyhow::Result;
use std::default::Default;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, trace};
use uuid::Uuid;

use tokio::sync::mpsc;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtcp;
use webrtc::rtp_transceiver::rtp_codec::{RTCRtpHeaderExtensionCapability, RTPCodecType};
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::rtp_transceiver::rtp_sender::RTCRtpSender;
use webrtc::rtp_transceiver::RTCRtpTransceiver;
use webrtc::track::track_remote::TrackRemote;

use crate::mediaengine;
use crate::signal::Signal;

// Peer ID unique to the connection/websocket
pub type Id = Uuid;

const TRANSPORT_TARGET_PUB: u32 = 0;
const TRANSPORT_TARGET_SUB: u32 = 1;

pub(super) type RtcpWriter = mpsc::Sender<Box<dyn rtcp::packet::Packet + Send + Sync>>;
pub(super) type RtcpReader = mpsc::Receiver<Box<dyn rtcp::packet::Packet + Send + Sync>>;

#[derive(Clone)]
pub struct ClientConfig {
    pub setting_engine: SettingEngine,
    pub rtc_config: RTCConfiguration,
    pub header_extensions: Vec<(RTCRtpHeaderExtensionCapability, RTPCodecType)>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            setting_engine: SettingEngine::default(),
            rtc_config: RTCConfiguration {
                ice_servers: vec![RTCIceServer {
                    urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                    ..Default::default()
                }],
                ..Default::default()
            },
            header_extensions: vec![],
        }
    }
}

pub struct Client<S: Signal> {
    pub signal: S,

    pub publisher: Arc<RTCPeerConnection>,
    pub pub_rtcp_writer: RtcpWriter,

    pub subscriber: Arc<RTCPeerConnection>,
    pub sub_rtcp_writer: RtcpWriter,

    pub sub_pending_candidates: Arc<Mutex<Vec<RTCIceCandidateInit>>>,

    config: ClientConfig,
}

impl<S> Client<S>
where
    S: Signal,
{
    async fn new(signal: S, config: ClientConfig) -> Result<Client<S>> {
        let (publisher, pub_rtcp_writer) = build_peer_connection(&config).await?;
        let (subscriber, sub_rtcp_writer) = build_peer_connection(&config).await?;

        Ok(Client {
            signal,
            publisher,
            pub_rtcp_writer,
            subscriber,
            sub_rtcp_writer,
            sub_pending_candidates: Arc::new(Mutex::new(vec![])),
            config,
        })
    }

    async fn join(&self, sid: String) -> Result<()> {
        //setup handlers
        //setup subscriber pc
        //send join command

        Ok(())
    }
}

/// Helper to build peer connections with the appropriate configuration
async fn build_peer_connection(cfg: &ClientConfig) -> Result<(Arc<RTCPeerConnection>, RtcpWriter)> {
    // Create a MediaEngine object to configure the supported codec
    let mut m = MediaEngine::default();
    mediaengine::register_default_codecs(&mut m)?;

    //for (capability, codec_type) in &cfg.header_extensions {
    //    m.register_header_extension(capability.clone(), codec_type.clone(), None)?;
    //}

    //#[cfg(feature = "simulcast")]
    //mediaengine::register_rtp_extension_simulcast(&mut m)?;

    //#[cfg(feature = "audiolevel")]
    //mediaengine::register_rtp_extension_audiolevel(&mut m)?;

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
    let peer_connection = Arc::new(api.new_peer_connection(cfg.rtc.clone()).await?);

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
