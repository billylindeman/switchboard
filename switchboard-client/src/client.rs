use std::sync::{Arc, Mutex};

use webrtc::{
    ice_transport::ice_candidate::RTCIceCandidateInit, peer_connection::RTCPeerConnection,
};

pub struct Client {
    pub publisher: Arc<RTCPeerConnection>,
    pub subscriber: Arc<RTCPeerConnection>,

    pub sub_pending_candidates: Arc<Mutex<Vec<RTCIceCandidateInit>>>,
}
