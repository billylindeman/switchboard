use uuid::Uuid;
use serde::{Serialize,Deserialize};


use std::collections::{HashMap};
use std::sync::{atomic, Arc, RwLock};

use jsonrpc_core::{Result, Error};
use jsonrpc_derive::rpc;
use jsonrpc_ws_server::{ServerBuilder,RequestContext};

use jsonrpc_pubsub::typed;
use jsonrpc_pubsub::{PubSubHandler, Session, SubscriptionId};


pub enum RoomEvent {
    StreamAdd { uuid: Uuid },
    StreamRemove { uuid: Uuid },
}

#[derive(Serialize,Deserialize,Debug)]
pub struct PublishRequest {
    pub request_id: Uuid,
    pub offer: String,
}

#[derive(Serialize,Deserialize,Debug)]
pub struct PublishReply {
    pub media_id: Uuid,
    pub answer: String,
}

#[derive(Serialize,Deserialize,Debug)]
pub struct SubscribeRequest{
    pub media_id: Uuid,
    pub offer: String,
}

#[derive(Serialize,Deserialize,Debug)]
pub struct SubscribeReply{
    pub media_id: Uuid,
    pub answer: String,
}

#[rpc]
pub trait SignalService {
    type Metadata;

    
	/// Join room 
	#[pubsub(subscription = "room", subscribe, name = "room_join")]
	fn join(&self, meta: Self::Metadata, subscriber: typed::Subscriber<String>, param: u64);

	/// Leave room
	#[pubsub(subscription = "room", unsubscribe, name = "room_leave")]
	fn leave(&self, meta: Option<Self::Metadata>, subscription: SubscriptionId) -> Result<bool>;

    #[rpc(name = "stream_list")]
    fn stream_list(&self) -> Result<Vec<String>>;

	#[rpc(name = "stream_publish")]
	fn stream_publish(&self, offer: String) -> Result<PublishReply>;

    #[rpc(name = "stream_subscribe")]
	fn stream_subscribe(&self, offer: String) -> Result<SubscribeRequest>;


}


#[derive(Default)]
pub struct Server {
    uid: atomic::AtomicUsize,
	active: Arc<RwLock<HashMap<SubscriptionId, typed::Sink<RoomEvent>>>>,
}

impl SignalService for Server {
	type Metadata = Arc<Session>;

	fn PublishStream(&self, req: PublishRequest) -> Result<PublishReply> {
        Err(Error::internal_error())
    }

	fn SubscribeStream(&self, req: SubscribeRequest) -> Result<SubscribeRequest> {
        Err(Error::internal_error())
    }

}

impl Server {
    pub fn run() {
        let mut io = PubSubHandler::default();
        let rpc = Server::default();
        let active_subscriptions = rpc.active.clone();
        io.extend_with(rpc.to_delegate());


        let builder = ServerBuilder::with_meta_extractor(io, |context: &RequestContext| {
			Arc::new(Session::new(context.sender().clone()))
        });

        let server = builder
            .start(&"0.0.0.0:3030".parse().unwrap())
            .unwrap();

        server.wait().expect("Server crashed")
    }
}


