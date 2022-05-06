use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::sfu;

#[derive(Serialize, Deserialize, Debug)]
pub struct JoinMsg {
    pub sid: String,
    pub offer: sfu::peer::SessionDescription,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JoinResponse {
    pub answer: sfu::peer::SessionDescription,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NegotiateMsg {
    pub desc: sfu::peer::SessionDescription,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrickleNotification {
    pub target: u32,
    pub candidate: sfu::peer::TrickleCandidate,
}

//#[derive(Debug)]
//pub enum SignalNotification {
//    Negotiate {
//        offer: SessionDescription,
//    },
//    Trickle {
//        target: u32,
//        candidate: TrickleCandidate,
//    },
//}
