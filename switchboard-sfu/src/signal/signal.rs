use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct JoinMsg {
    pub sid: String,
    pub offer: SessionDescription,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JoinResponse {
    pub answer: SessionDescription,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NegotiateMsg {
    pub desc: SessionDescription,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrickleNotification {
    pub target: u32,
    pub candidate: TrickleCandidate,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionDescription {
    #[serde(rename = "type")]
    pub t: String,
    pub sdp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrickleCandidate {
    pub candidate: String,
    #[serde(rename = "sdpMid")]
    pub sdp_mid: Option<String>,
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_mline_index: u32,
}

#[derive(Debug)]
pub enum SignalNotification {
    Negotiate {
        offer: SessionDescription,
    },
    Trickle {
        target: u32,
        candidate: TrickleCandidate,
    },
}
