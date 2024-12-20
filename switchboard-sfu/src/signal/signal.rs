use anyhow::Result;
use enclose::enc;
use futures_channel::{mpsc, oneshot};
use futures_util::StreamExt;
use log::*;
use serde_json::Value;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use switchboard_proto as proto;

pub enum Event {
    JoinRequest(
        oneshot::Sender<proto::signal::JoinResponse>,
        proto::signal::JoinMsg,
    ),
    PublisherOffer(
        oneshot::Sender<RTCSessionDescription>,
        proto::signal::NegotiateMsg,
    ),
    SubscriberOffer(RTCSessionDescription),
    SubscriberAnswer(proto::signal::NegotiateMsg),
    TrickleIce(proto::signal::TrickleNotification),
    Presence(proto::signal::Presence),
}

pub type ReadStream = mpsc::UnboundedReceiver<Result<Event>>;
pub type WriteStream = mpsc::UnboundedSender<Result<Event>>;

/// Process json::Event streams into signal::Event streams
pub async fn handle_messages(
    mut rpc_read: super::jsonrpc::ReadStream,
    rpc_write: super::jsonrpc::WriteStream,
) -> (ReadStream, WriteStream) {
    let (sig_read_tx, sig_read_rx) = mpsc::unbounded::<Result<Event>>();
    let (sig_write_tx, mut sig_write_rx) = mpsc::unbounded::<Result<Event>>();

    tokio::spawn(enc!( (rpc_write) async move {
        while let Some(Ok(rpc)) = rpc_read.next().await {
            match rpc {
                proto::jsonrpc::Event::Request(r) => match r.method.as_str() {
                    "join" => {
                        let id = r.id;
                        let (tx, rx) = oneshot::channel::<proto::signal::JoinResponse>();

                        info!("got join request");

                        tokio::spawn(enc!( (rpc_write) async move {
                            let result = rx.await.unwrap();
                            let response = proto::jsonrpc::Response{
                                id: id,
                                result: Some(serde_json::from_value(serde_json::to_value(result).unwrap()).expect("error creating response")),
                                error: None
                            };

                            rpc_write.unbounded_send(Ok(proto::jsonrpc::Event::Response(response))).expect("error sending response");
                        }));

                        let j: proto::signal::JoinMsg =
                            serde_json::from_value(Value::Object(r.params)).expect("error parsing");

                        sig_read_tx.unbounded_send(Ok(Event::JoinRequest(tx, j))).expect("error forwarding signal message");
                    }
                    "offer" => {
                        let id = r.id;
                        let (tx, rx) = oneshot::channel::<RTCSessionDescription>();

                        info!("got publisher negotiation offer");

                        tokio::spawn(enc!( (rpc_write) async move {
                            let result = rx.await.unwrap();
                            let response = proto::jsonrpc::Response{
                                id: id,
                                result: Some(serde_json::from_value(serde_json::to_value(result).unwrap()).expect("error creating response")),
                                error: None
                            };

                            rpc_write.unbounded_send(Ok(proto::jsonrpc::Event::Response(response))).expect("error sending response");
                        }));

                        let o: proto::signal::NegotiateMsg =
                            serde_json::from_value(Value::Object(r.params)).expect("error parsing");

                        sig_read_tx.unbounded_send(Ok(Event::PublisherOffer(tx, o))).expect("error forwarding signal message");
                    },
                    "presence_set" => {
                        let meta: serde_json::Value =
                            serde_json::from_value(Value::Object(r.params)).expect("error parsing");
                        sig_read_tx.unbounded_send(Ok(Event::Presence(proto::signal::Presence{
                            revision: 0,
                            meta: meta,
                        }))).expect("error forwarding signal message");
                    }

                    _ => {}
                },
                proto::jsonrpc::Event::Notification(n) => match n.method.as_str() {
                    "trickle" => {
                        info!("got trickle notification");
                        let t: proto::signal::TrickleNotification
                            = serde_json::from_value(Value::Object(n.params)).expect("error parsing");
                        sig_read_tx.unbounded_send(Ok(Event::TrickleIce(t))).expect("error forwarding signal message");
                    },
                    "answer" => {
                        let n: proto::signal::NegotiateMsg =
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
                    let n = proto::jsonrpc::Notification{
                        method: "trickle".to_owned(),
                        params: serde_json::from_value(serde_json::to_value(ice).unwrap()).unwrap(),
                    };
                    rpc_write.unbounded_send(Ok(proto::jsonrpc::Event::Notification(n))).expect("error sending notification");
                }
                Event::SubscriberOffer(offer) => {
                    let n = proto::jsonrpc::Notification{
                        method: "offer".to_owned(),
                        params: serde_json::from_value(serde_json::to_value(offer).unwrap()).unwrap(),
                    };
                    rpc_write.unbounded_send(Ok(proto::jsonrpc::Event::Notification(n))).expect("error sending notification");
                }
                Event::Presence(presence) => {
                    let n = proto::jsonrpc::Notification {
                        method: "presence".to_owned(),
                        params: serde_json::from_value(serde_json::to_value(presence).unwrap()).unwrap(),
                    };
                    rpc_write.unbounded_send(Ok(proto::jsonrpc::Event::Notification(n))).expect("error sending notification");
                }
                _ => {}
            }
        }
    }));

    (sig_read_rx, sig_write_tx)
}
