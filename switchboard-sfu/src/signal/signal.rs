use anyhow::Result;
use enclose::enc;
use futures_channel::{mpsc, oneshot};
use futures_util::StreamExt;
use log::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use super::jsonrpc;

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

pub enum Event {
    JoinRequest(oneshot::Sender<JoinResponse>, JoinMsg),
    PublisherOffer(oneshot::Sender<RTCSessionDescription>, NegotiateMsg),
    SubscriberOffer(RTCSessionDescription),
    SubscriberAnswer(NegotiateMsg),
    TrickleIce(TrickleNotification),
    Presence(Presence),
}

pub type ReadStream = mpsc::UnboundedReceiver<Result<Event>>;
pub type WriteStream = mpsc::UnboundedSender<Result<Event>>;

/// Process json::Event streams into signal::Event streams
pub async fn handle_messages(
    mut rpc_read: jsonrpc::ReadStream,
    rpc_write: jsonrpc::WriteStream,
) -> (ReadStream, WriteStream) {
    let (sig_read_tx, sig_read_rx) = mpsc::unbounded::<Result<Event>>();
    let (sig_write_tx, mut sig_write_rx) = mpsc::unbounded::<Result<Event>>();

    tokio::spawn(enc!( (rpc_write) async move {
        while let Some(Ok(rpc)) = rpc_read.next().await {
            match rpc {
                jsonrpc::Event::Request(r) => match r.method.as_str() {
                    "join" => {
                        let id = r.id;
                        let (tx, rx) = oneshot::channel::<JoinResponse>();

                        info!("got join request");

                        tokio::spawn(enc!( (rpc_write) async move {
                            let result = rx.await.unwrap();
                            let response = jsonrpc::Response{
                                id: id,
                                result: Some(serde_json::from_value(serde_json::to_value(result).unwrap()).expect("error creating response")),
                                error: None
                            };

                            rpc_write.unbounded_send(Ok(jsonrpc::Event::Response(response))).expect("error sending response");
                        }));

                        let j: JoinMsg =
                            serde_json::from_value(Value::Object(r.params)).expect("error parsing");

                        sig_read_tx.unbounded_send(Ok(Event::JoinRequest(tx, j))).expect("error forwarding signal message");
                    }
                    "offer" => {
                        let id = r.id;
                        let (tx, rx) = oneshot::channel::<RTCSessionDescription>();

                        info!("got publisher negotiation offer");

                        tokio::spawn(enc!( (rpc_write) async move {
                            let result = rx.await.unwrap();
                            let response = jsonrpc::Response{
                                id: id,
                                result: Some(serde_json::from_value(serde_json::to_value(result).unwrap()).expect("error creating response")),
                                error: None
                            };

                            rpc_write.unbounded_send(Ok(jsonrpc::Event::Response(response))).expect("error sending response");
                        }));

                        let o: NegotiateMsg =
                            serde_json::from_value(Value::Object(r.params)).expect("error parsing");

                        sig_read_tx.unbounded_send(Ok(Event::PublisherOffer(tx, o))).expect("error forwarding signal message");
                    },
                    "presence_set" => {
                        let meta: serde_json::Value =
                            serde_json::from_value(Value::Object(r.params)).expect("error parsing");
                        sig_read_tx.unbounded_send(Ok(Event::Presence(Presence{
                            revision: 0,
                            meta: meta,
                        }))).expect("error forwarding signal message");
                    }

                    _ => {}
                },
                jsonrpc::Event::Notification(n) => match n.method.as_str() {
                    "trickle" => {
                        info!("got trickle notification");
                        let t: TrickleNotification
                            = serde_json::from_value(Value::Object(n.params)).expect("error parsing");
                        sig_read_tx.unbounded_send(Ok(Event::TrickleIce(t))).expect("error forwarding signal message");
                    },
                    "answer" => {
                        let n: NegotiateMsg =
                            serde_json::from_value(Value::Object(n.params)).expect("error parsing");
                        sig_read_tx.unbounded_send(Ok(Event::SubscriberAnswer(n))).expect("error forwarding signal message");
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }));

    tokio::spawn(enc!( (rpc_write) async move {
        while let Some(Ok(evt)) = sig_write_rx.next().await {
            match evt {
                Event::TrickleIce(ice) => {
                    let n = jsonrpc::Notification{
                        method: "trickle".to_owned(),
                        params: serde_json::from_value(serde_json::to_value(ice).unwrap()).unwrap(),
                    };
                    rpc_write.unbounded_send(Ok(jsonrpc::Event::Notification(n))).expect("error sending notification");
                }
                Event::SubscriberOffer(offer) => {
                    let n = jsonrpc::Notification{
                        method: "offer".to_owned(),
                        params: serde_json::from_value(serde_json::to_value(offer).unwrap()).unwrap(),
                    };
                    rpc_write.unbounded_send(Ok(jsonrpc::Event::Notification(n))).expect("error sending notification");
                }
                Event::Presence(presence) => {
                    let n = jsonrpc::Notification {
                        method: "presence".to_owned(),
                        params: serde_json::from_value(serde_json::to_value(presence).unwrap()).unwrap(),
                    };
                    rpc_write.unbounded_send(Ok(jsonrpc::Event::Notification(n))).expect("error sending notification");
                }
                _ => {}
            }
        }
    }));

    (sig_read_rx, sig_write_tx)
}
