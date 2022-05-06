use anyhow::Result;
use serde::{Deserialize, Serialize};

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

pub trait Signal {
    fn join(sid: String, offer: SessionDescription) -> Result<SessionDescription>;
    fn subscriber_answer_for_offer(offer: SessionDescription) -> Result<SessionDescription>;
}
