use gst::prelude::*;
use log::*;
use uuid::Uuid;

use failure::Error;
use std::collections::HashMap;
use std::result::Result;
use std::sync::{Arc, RwLock};

use crate::peer::PeerConnection;

pub struct Broadcast {
    pub id: Uuid,
    pub peer: PeerConnection,
    pub subscribers: HashMap<Uuid, Arc<PeerConnection>>,

    pipeline: gst::Pipeline,
}

pub struct SFU {
    pub pipeline: gst::Pipeline,
    pub broadcasts: HashMap<Uuid, Arc<Broadcast>>,
}

impl SFU {
    pub fn default() -> Result<SFU, Error> {
        let pipeline = gst::Pipeline::new(Some("SFU"));

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

        Ok(SFU {
            pipeline: pipeline,
            broadcasts: HashMap::new(),
        })
    }

    pub fn create_broadcast(&mut self) -> Result<Arc<PeerConnection>, Error> {
        let broadcast_id = Uuid::new_v4();
        let peer = Arc::new(PeerConnection::new(&self.pipeline, broadcast_id)?);
        self.broadcasts.insert(broadcast_id, peer.clone());

        Ok(peer)
    }

    pub fn create_subscription(
        &mut self,
        broadcast_id: Uuid,
    ) -> Result<Arc<PeerConnection>, Error> {
        if let Some(broadcast) = self.broadcasts.get_mut(&broadcast_id) {
            let subscription_id = Uuid::new_v4();
            let peer = Arc::new(PeerConnection::new(&broadcast.pipeline, subscription_id)?);
            broadcast.subscribers.insert(subscription_id, peer);

            Ok(peer)
        }

        Err(format_err!("broadcast not found {}", broadcast_id))
    }
}
