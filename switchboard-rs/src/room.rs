use gst::prelude::*;

use std::result::Result;
use failure::Error;

pub type Rooms Arc<RwLock<HashMap<u64,Room>>>;


pub struct Room {
    pub id: u64,
    pub pipeline: gst::Pipeline,

    pub broadcasts: Vec<Broadcast>,
    pub subscriptions: Vec<Subscription>
}

impl Room {
    pub fn new(room_id: u64) -> Result<Room, Error> {
        let pipeline = gst::Pipeline::new(Some(&format!("RoomPipeline{}", room_id)));

        // Asynchronously set the pipeline to Playing
        pipeline
            .set_state(gst::State::Playing)
            .expect("Couldn't set pipeline to Playing");

        Ok(Room {
            id: room_id,
            pipeline: pipeline,
            broadcasts: vec![],
            subscriptions: vec![],
        })
    }

}

pub struct Peer {
    pub id: i64,
}

pub struct Broadcast {
    pub id: i64,
    pub peer_id: i64,
    pub webrtcbin: gst::Element,
}

pub struct Subscription {
    pub broadcast_id: i64,
    pub peer_id: i64,

    pub webrtcbin: gst::Element,
}

