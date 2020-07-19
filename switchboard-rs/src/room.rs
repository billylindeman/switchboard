use gst::prelude::*;

use uuid::Uuid;

use log::*;

use std::sync::{Arc,RwLock};
use std::collections::{HashMap,HashSet};
use std::result::Result;
use failure::Error;


use crate::peer::PeerConnection;

#[derive(Default)]
pub struct RoomController(HashMap<Uuid,Arc<RwLock<Room>>>);

impl RoomController {
    pub fn get_or_create_room(&self, uuid: Uuid) -> Arc<RwLock<Room>> {
        match self.0.get(&uuid) {
            Some(room) => room.clone(),
            _ => {
                let room = Arc::new(
                    RwLock::new(
                        Room::new(uuid).expect("error creating room")
                    )
                );
                room
            }
        }
    }
}

pub struct Room {
    pub id: Uuid,
    pub pipeline: gst::Pipeline,

    pub peers: HashMap<Uuid, Arc<PeerConnection>>,

    pub broadcasts: HashSet<Uuid>,//broadcast_id
    pub subscriptions: HashMap<Uuid, Uuid>, //subscription_id, peer_id
}

impl Room {
    pub fn new(room_id: Uuid) -> Result<Room, Error> {
        let pipeline = gst::Pipeline::new(Some(&format!("RoomPipeline{}", room_id)));

        // Asynchronously set the pipeline to Playing
        pipeline
            .set_state(gst::State::Playing)
            .expect("Couldn't set pipeline to Playing");

        let bus = pipeline.get_bus().unwrap();
        bus.add_signal_watch();
        bus.connect_message(move |_, msg| {
            use gst::MessageView;

            match msg.view() {
                MessageView::StateChanged(s) => debug!("state changed: {} {:?}", s.get_src().unwrap().get_name(), s),
                MessageView::Eos(..) => (),
                MessageView::Warning(warn) =>{
                    warn!("{} {:?} ", warn.get_error(), warn.get_debug());
                },
                MessageView::Error(err) => {
                    error!("{} {:?} ", err.get_error(), err.get_debug());
                    panic!("Pipeline Broken");
                },
                MessageView::Info(info) => {
                    info!("{} {:?} ", info.get_error(), info.get_debug());
                },
                _ => (),
            }
        });

        Ok(Room {
            id: room_id,
            pipeline: pipeline,
            peers: HashMap::new(),
            broadcasts: HashSet::new(),
            subscriptions: HashMap::new(),
        })
    }

    pub fn publish(&self, offer: String) -> Result<(Uuid, Arc<PeerConnection>), Error> {
        let stream_id = Uuid::new_v4();
        let peer = Arc::new(PeerConnection::new(&self.pipeline, stream_id)?); 

        self.peers.insert(stream_id, peer.clone());
        self.broadcasts.insert(stream_id);

        Ok((stream_id, peer))
    }

    pub fn subscribe(&self, broadcast_id: Uuid, offer: String) -> Result<(Uuid, Arc<PeerConnection>), Error> {
        let subscriber_id = Uuid::new_v4();
        let peer = Arc::new(PeerConnection::new(&self.pipeline, subscriber_id)?); 

        self.peers.insert(subscriber_id, peer.clone());
        self.subscriptions.insert(subscriber_id, broadcast_id);

        Ok((subscriber_id, peer))
    }


}

pub struct Broadcast {
    pub id: Uuid,
    pub peer: PeerConnection,
}

pub struct Subscription {
    pub id: Uuid,
    pub src: PeerConnection,
    pub dest: PeerConnection,
}

