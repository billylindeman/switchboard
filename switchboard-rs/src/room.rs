use gst::prelude::*;

use uuid::Uuid;

use log::*;

use std::sync::{Arc,RwLock};
use std::collections::{HashMap};
use std::result::Result;
use failure::Error;


use crate::peer::PeerConnection;

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

    pub peers: Vec<Arc<PeerConnection>>,

    pub broadcasts: Vec<Broadcast>,
    pub subscriptions: Vec<Subscription>
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
            peers: vec![],
            broadcasts: vec![],
            subscriptions: vec![],
        })
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

