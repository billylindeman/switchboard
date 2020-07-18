use uuid::Uuid;
use serde::{Serialize,Deserialize};

use std::collections::{HashMap};
use std::thread;
use std::sync::{atomic, Arc, RwLock};

use jsonrpc_core::futures::Future;
use jsonrpc_core::{Result, Error, ErrorCode};
use jsonrpc_derive::rpc;
use jsonrpc_ws_server::{ServerBuilder,RequestContext};

use jsonrpc_pubsub::typed;
use jsonrpc_pubsub::{PubSubHandler, Session, SubscriptionId};

use crate::room;

#[derive(Serialize,Deserialize,Debug)]
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
	fn room_join(&self, meta: Self::Metadata, subscriber: typed::Subscriber<RoomEvent>, param: u64);

	/// Leave room
	#[pubsub(subscription = "room", unsubscribe, name = "room_leave")]
	fn room_leave(&self, meta: Option<Self::Metadata>, subscription: SubscriptionId) -> Result<bool>;

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
    rooms: room::RoomController,
}


impl SignalService for Server {
	type Metadata = Arc<Session>;

	fn room_join(&self, _meta: Self::Metadata, subscriber: typed::Subscriber<RoomEvent>, room_id: Uuid) {
        let room = self.rooms.get_or_create_room(room_id);

//			subscriber
//				.reject(Error {
//					code: ErrorCode::InvalidParams,
//					message: "Rejecting subscription - invalid parameters provided.".into(),
//					data: None,
//				})
//				.unwrap();
//			return;

		let id = self.uid.fetch_add(1, atomic::Ordering::SeqCst);
		let sub_id = SubscriptionId::Number(id as u64);
		let sink = subscriber.assign_id(sub_id.clone()).unwrap();
		self.active.write().unwrap().insert(sub_id, sink);
	}

	fn room_leave(&self, _meta: Option<Self::Metadata>, id: SubscriptionId) -> Result<bool> {
		let removed = self.active.write().unwrap().remove(&id);
		if removed.is_some() {
			Ok(true)
		} else {
			Err(Error {
				code: ErrorCode::InvalidParams,
				message: "Invalid subscription.".into(),
				data: None,
			})
		}
	}

    fn stream_list(&self) -> Result<Vec<String>> {
        Ok([].into())
    }

	fn stream_publish(&self, offer: String) -> Result<PublishReply> {
        Err(Error::internal_error())
    }

	fn stream_subscribe(&self, offer: String) -> Result<SubscribeRequest> {
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
				let _ = sink.notify(Ok(
                    RoomEvent::StreamAdd{ uuid: Uuid::new_v4()}
                )).wait();
			}
		    thread::sleep(::std::time::Duration::from_secs(1));
        });

        let builder = ServerBuilder::with_meta_extractor(io, |context: &RequestContext| {
			Arc::new(Session::new(context.sender().clone()))
        });

        let server = builder
            .start(&"0.0.0.0:3030".parse().unwrap())
            .unwrap();

        server.wait().expect("Server crashed")
    }
}


