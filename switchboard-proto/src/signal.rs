use serde::{Deserialize, Serialize};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

#[derive(Serialize, Deserialize, Debug)]
pub struct JoinMsg {
    pub sid: String,
    pub offer: RTCSessionDescription,
}

pub type JoinResponse = RTCSessionDescription;

#[derive(Serialize, Deserialize, Debug)]
pub struct NegotiateMsg {
    pub desc: RTCSessionDescription,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrickleNotification {
    pub target: u32,
    pub candidate: TrickleCandidate,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrickleCandidate {
    pub candidate: String,
    #[serde(rename = "sdpMid")]
    pub sdp_mid: Option<String>,
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_mline_index: u32,
}

impl From<RTCIceCandidateInit> for TrickleCandidate {
    fn from(t: RTCIceCandidateInit) -> TrickleCandidate {
        TrickleCandidate {
            candidate: t.candidate,
            sdp_mid: t.sdp_mid,
            sdp_mline_index: t.sdp_mline_index.unwrap() as u32,
        }
    }
}

impl From<TrickleCandidate> for RTCIceCandidateInit {
    fn from(t: TrickleCandidate) -> RTCIceCandidateInit {
        RTCIceCandidateInit {
            candidate: t.candidate,
            sdp_mid: t.sdp_mid,
            sdp_mline_index: Some(t.sdp_mline_index as u16),
            username_fragment: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Presence {
    pub revision: u64,
    pub meta: serde_json::Value,
}
