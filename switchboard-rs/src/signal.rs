use serde::{Deserialize, Serialize};
use serde_json::Map;
use uuid::Uuid;

use std::collections::HashMap;
use std::sync::{atomic, Arc, RwLock};
use std::thread;

use failure::*;
use log::*;

use jsonrpc_core::futures::future::{self, BoxFuture};
use jsonrpc_core::futures::{lazy, Future};
use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use jsonrpc_ws_server::tokio;
use jsonrpc_ws_server::{RequestContext, ServerBuilder};

use jsonrpc_pubsub::typed;
use jsonrpc_pubsub::{PubSubHandler, PubSubMetadata, SubscriptionId};

use crate::peer;
use crate::router;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", untagged)]
pub enum RoomEvent {
    Peer(peer::PeerEvent),
}

#[serde(tag = "type", rename_all = "lowercase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SDP {
    Offer { sdp: String },
    Answer { sdp: String },
}

#[rpc(server)]
pub trait SignalService {
    type Metadata;

    /// Join room
    #[pubsub(subscription = "room", subscribe, name = "join")]
    fn room_join(
        &self,
        session: Self::Metadata,
        subscriber: typed::Subscriber<RoomEvent>,
        sid: Uuid,
    );

    /// Leave room
    #[pubsub(subscription = "room", unsubscribe, name = "leave")]
    fn room_leave(
        &self,
        session: Option<Self::Metadata>,
        subscription: SubscriptionId,
    ) -> Result<bool>;

    #[rpc(meta, name = "offer")]
    fn offer(&self, session: Self::Metadata, sdp: SDP) -> Result<()>;

    #[rpc(meta, name = "answer")]
    fn answer(&self, session: Self::Metadata, sdp: SDP) -> Result<()>;

    #[rpc(meta, name = "trickle")]
    fn trickle(
        &self,
        session: Self::Metadata,
        sdp_mline_index: u32,
        candidate: String,
    ) -> Result<()>;
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
    presence:
        Arc<RwLock<HashMap<PeerID, (Arc<peer::PeerConnection>, Arc<RwLock<router::Router>>)>>>,
}

impl SignalService for Server {
    type Metadata = Session;

    fn room_join(
        &self,
        session: Self::Metadata,
        subscriber: typed::Subscriber<RoomEvent>,
        room_id: RoomID,
    ) {
        info!("[peer {}] room_join: {}", session.peer_id, room_id);

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
        let sub_id = SubscriptionId::String(session.peer_id.to_string());
        let sink = subscriber.assign_id(sub_id.clone()).unwrap();
        self.active.write().unwrap().insert(sub_id, sink.clone());

        let peer = router.write().unwrap().join(session.peer_id).unwrap();
        self.presence
            .write()
            .unwrap()
            .insert(session.peer_id, (peer.clone(), router.clone()));

        let rx = peer.rx.clone();
        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                debug!("sending msg: {:#?}", msg);
                sink.notify(Ok(RoomEvent::Peer(msg)))
                    .wait()
                    .expect("Error sending notification");
            }
        });
    }

    fn room_leave(&self, _session: Option<Self::Metadata>, id: SubscriptionId) -> Result<bool> {
        let removed = self.active.write().unwrap().remove(&id);
        if removed.is_some() {
            if let SubscriptionId::String(peer_id) = id {
                info!("[peer {}] left room", peer_id);
                self.presence
                    .write()
                    .unwrap()
                    .remove(&Uuid::parse_str(&peer_id).unwrap());
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

    fn offer(&self, session: Self::Metadata, sdp: SDP) -> Result<()> {
        debug!("peer got offer");
        let presence = self.presence.read().unwrap();
        let (peer, _) = presence.get(&session.peer_id).unwrap();
        peer.set_remote_description(sdp);
        thread::sleep_ms(1000);
        peer.create_answer();
        Ok(())
    }

    fn answer(&self, session: Self::Metadata, sdp: SDP) -> Result<()> {
        debug!("peer got answer");
        let presence = self.presence.read().unwrap();
        let (peer, _) = presence.get(&session.peer_id).unwrap();
        peer.set_remote_description(sdp);

        Ok(())
    }

    fn trickle(
        &self,
        session: Self::Metadata,
        sdp_mline_index: u32,
        candidate: String,
    ) -> Result<()> {
        debug!("peer trickle ice");
        let presence = self.presence.read().unwrap();
        let (peer, _) = presence.get(&session.peer_id).unwrap();

        peer.add_ice_candidate(sdp_mline_index, candidate)
            .expect("error adding ice candidate");

        Ok(())
    }
    //fn stream_publish(&self, session: Self::Metadata, sdp: SDP) -> BoxFuture<PublishReply, Error> {
    //    if let SDP::Offer { sdp: _ } = sdp {
    //        debug!("got sdp {:#?}", &sdp);
    //        if let Some(router) = self.presence.read().unwrap().get(&session.peer_id) {
    //            let (broadcast_id, peer) = router.write().unwrap().publish(sdp).unwrap();
    //            debug!("created broadcast: {}", broadcast_id);

    //            return lazy(move || {
    //                debug!("bruh");

    //                future::ok(PublishReply {
    //                    media_id: broadcast_id,
    //                    answer: "".to_owned(),
    //                })
    //            })
    //            .boxed();
    //        }
    //    }

    //    future::failed(Error::internal_error()).boxed()
    //}
}

impl Server {
    pub fn run() {
        let mut io = PubSubHandler::default();
        let rpc = Server::default();

        let active_subscriptions = rpc.active.clone();
        io.extend_with(rpc.to_delegate());

        //thread::spawn(move || loop {
        //    let subscribers = active_subscriptions.read().unwrap();
        //    for sink in subscribers.values() {
        //        let _ = sink
        //            .notify(Ok(RoomEvent::StreamAdd {
        //                uuid: Uuid::new_v4(),
        //            }))
        //            .wait();
        //    }
        //    thread::sleep(::std::time::Duration::from_secs(1));
        //});

        let builder = ServerBuilder::with_meta_extractor(io, |context: &RequestContext| Session {
            peer_id: Uuid::new_v4(),
            session: Arc::new(jsonrpc_pubsub::Session::new(context.sender().clone())),
        });

        let server = builder.start(&"0.0.0.0:3030".parse().unwrap()).unwrap();

        server.wait().expect("Server crashed")
    }
}
