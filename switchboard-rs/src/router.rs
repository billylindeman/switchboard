use gst::prelude::*;

use uuid::Uuid;

use log::*;

use failure::Error;
use std::collections::{HashMap, HashSet};
use std::result::Result;
use std::sync::{Arc, RwLock};

use crate::peer::PeerConnection;
use crate::signal;

#[derive(Default)]
pub struct RouterMap(Arc<RwLock<HashMap<Uuid, Arc<RwLock<Router>>>>>);

impl RouterMap {
    pub fn get_or_create_router(&self, uuid: Uuid) -> Arc<RwLock<Router>> {
        match self.0.write().unwrap().get(&uuid) {
            Some(room) => room.clone(),
            _ => {
                let room = Arc::new(RwLock::new(Router::new(uuid).expect("error creating room")));

                room
            }
        }
    }
}
/// The Router is responsible for managing a gstreamer pipeline and connecting / disconnecting
/// all respective webrtcbins (representing PeerConnections)
pub struct Router {
    pub id: Uuid,
    pub pipeline: gst::Pipeline,

    pub peers: HashMap<Uuid, Arc<PeerConnection>>,
}

impl Router {
    pub fn new(room_id: Uuid) -> Result<Router, Error> {
        let pipeline = gst::Pipeline::new(Some(&format!("RouterPipeline{}", room_id)));

        // Asynchronously set the pipeline to Playing
        pipeline
            .set_state(gst::State::Playing)
            .expect("Couldn't set pipeline to Playing");

        let bus = pipeline.get_bus().unwrap();
        bus.add_signal_watch();
        bus.connect_message(move |_, msg| {
            use gst::MessageView;

            match msg.view() {
                MessageView::StateChanged(s) => {
                    debug!("state changed: {} {:?}", s.get_src().unwrap().get_name(), s)
                }
                MessageView::Eos(..) => (),
                MessageView::Warning(warn) => {
                    warn!("{} {:?} ", warn.get_error(), warn.get_debug());
                }
                MessageView::Error(err) => {
                    error!("{} {:?} ", err.get_error(), err.get_debug());
                    panic!("Pipeline Broken");
                }
                MessageView::Info(info) => {
                    info!("{} {:?} ", info.get_error(), info.get_debug());
                }
                _ => (),
            }
        });

        Ok(Router {
            id: room_id,
            pipeline: pipeline,
            peers: HashMap::new(),
        })
    }

    pub fn join(&mut self, peer_id: Uuid) -> Result<Arc<PeerConnection>, Error> {
        let peer = Arc::new(PeerConnection::new(&self.pipeline, peer_id)?);
        self.peers.insert(peer_id, peer.clone());
        Ok(peer)
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
