use serde::{Deserialize, Serialize};
use serde_json::Map;
use uuid::Uuid;

use std::collections::HashMap;
use std::sync::{atomic, Arc, RwLock};
use std::thread;

use log::*;

use jsonrpc_core::futures::future::{self, FutureResult};
use jsonrpc_core::futures::Future;
use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use jsonrpc_ws_server::{RequestContext, ServerBuilder};

use jsonrpc_pubsub::typed;
use jsonrpc_pubsub::{PubSubHandler, PubSubMetadata, SubscriptionId};

use crate::peer;
use crate::router;

#[derive(Serialize, Deserialize, Debug)]
pub enum RoomEvent {
    StreamAdd {
        uuid: Uuid,
    },
    StreamRemove {
        uuid: Uuid,
    },
    PeerMsg {
        to: Uuid,
        from: Uuid,
        event: peer::PeerEvent,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishReply {
    pub media_id: Uuid,
    pub answer: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubscribeReply {
    pub media_id: Uuid,
    pub answer: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SDP {
    Offer { sdp: String },
    Answer { sdp: String },
}

#[rpc]
pub trait SignalService {
    type Metadata;

    /// Join room
    #[pubsub(subscription = "room", subscribe, name = "room_join")]
    fn room_join(
        &self,
        session: Self::Metadata,
        subscriber: typed::Subscriber<RoomEvent>,
        room_id: Uuid,
    );

    /// Leave room
    #[pubsub(subscription = "room", unsubscribe, name = "room_leave")]
    fn room_leave(
        &self,
        session: Option<Self::Metadata>,
        subscription: SubscriptionId,
    ) -> Result<bool>;
    #[rpc(meta, name = "stream_list")]
    fn stream_list(&self, session: Self::Metadata) -> Result<Vec<String>>;

    #[rpc(meta, name = "stream_publish")]
    fn stream_publish(
        &self,
        session: Self::Metadata,
        stream_id: String,
        sdp: SDP,
    ) -> FutureResult<PublishReply, Error>;

    #[rpc(meta, name = "stream_subscribe")]
    fn stream_subscribe(&self, session: Self::Metadata, sdp: SDP) -> Result<SubscribeReply>;
}

type PeerID = Uuid;
type RoomID = Uuid;

#[derive(Clone)]
pub struct Session {
    session: Arc<jsonrpc_pubsub::Session>,
    peer_id: PeerID,
}

impl jsonrpc_core::Metadata for Session {}
impl PubSubMetadata for Session {
    fn session(&self) -> Option<Arc<jsonrpc_pubsub::Session>> {
        Some(self.session.clone())
    }
}

#[derive(Default)]
pub struct Server {
    active: Arc<RwLock<HashMap<SubscriptionId, typed::Sink<RoomEvent>>>>,
    routers: router::RouterMap,
    presence: Arc<RwLock<HashMap<PeerID, Arc<RwLock<router::Router>>>>>,
}

impl SignalService for Server {
    type Metadata = Session;

    fn room_join(
        &self,
        session: Self::Metadata,
        subscriber: typed::Subscriber<RoomEvent>,
        room_id: RoomID,
    ) {
        info!("room_join: {}", room_id);

        if let Some(_) = self.presence.read().unwrap().get(&session.peer_id) {
            subscriber
                .reject(Error {
                    code: ErrorCode::InvalidParams,
                    message: "Cannot join room while in another room".into(),
                    data: None,
                })
                .unwrap();
            return;
        }

        let router = self.routers.get_or_create_router(room_id);
        self.presence
            .write()
            .unwrap()
            .insert(session.peer_id, router.clone());

        let sub_id = SubscriptionId::String(session.peer_id.to_string());
        let sink = subscriber.assign_id(sub_id.clone()).unwrap();
        self.active.write().unwrap().insert(sub_id, sink);
    }

    fn room_leave(&self, session: Option<Self::Metadata>, id: SubscriptionId) -> Result<bool> {
        let removed = self.active.write().unwrap().remove(&id);
        if removed.is_some() {
            if let Some(session) = session {
                self.presence.write().unwrap().remove(&session.peer_id);
            }
            Ok(true)
        } else {
            Err(Error {
                code: ErrorCode::InvalidParams,
                message: "Invalid subscription.".into(),
                data: None,
            })
        }
    }

    fn stream_list(&self, session: Self::Metadata) -> Result<Vec<String>> {
        Ok([].into())
    }

    fn stream_publish(
        &self,
        session: Self::Metadata,
        stream_id: String,
        sdp: SDP,
    ) -> FutureResult<PublishReply, Error> {
        if let SDP::Offer { sdp } = sdp {
            debug!("got sdp {}", sdp);
            if let Some(room) = self.presence.read().unwrap().get(&session.peer_id) {
                let (broadcast_id, peer) = room.write().unwrap().publish(sdp).unwrap();
                debug!("created broadcast: {}", broadcast_id);

                return future::finished(PublishReply {
                    media_id: broadcast_id,
                    answer: "".to_owned(),
                });
            }
        }

        future::failed(Error::internal_error())
    }

    fn stream_subscribe(&self, session: Self::Metadata, sdp: SDP) -> Result<SubscribeReply> {
        Err(Error::internal_error())
    }
}

impl Server {
    pub fn run() {
        let mut io = PubSubHandler::default();
        let rpc = Server::default();
        let active_subscriptions = rpc.active.clone();
        io.extend_with(rpc.to_delegate());

        thread::spawn(move || loop {
            let subscribers = active_subscriptions.read().unwrap();
            for sink in subscribers.values() {
                let _ = sink
                    .notify(Ok(RoomEvent::StreamAdd {
                        uuid: Uuid::new_v4(),
                    }))
                    .wait();
            }
            thread::sleep(::std::time::Duration::from_secs(1));
        });

        let builder = ServerBuilder::with_meta_extractor(io, |context: &RequestContext| Session {
            peer_id: Uuid::new_v4(),
            session: Arc::new(jsonrpc_pubsub::Session::new(context.sender().clone())),
        });

        let server = builder.start(&"0.0.0.0:3030".parse().unwrap()).unwrap();

        server.wait().expect("Server crashed")
    }
}
