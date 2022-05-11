use anyhow::{format_err, Result};
use async_mutex::Mutex;
use async_trait::async_trait;
use enclose::enc;
use futures::StreamExt;
use futures_channel::mpsc;
use log::*;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::sfu::peer;
use crate::sfu::routing::MediaTrackRouterHandle;

// SessionID represents a collection of peers that can route tracks to eachother
pub type Id = String;
pub type SessionHandle<T> = Arc<T>;

pub type ReadStream = mpsc::Receiver<SessionEvent>;
pub type WriteStream = mpsc::Sender<SessionEvent>;

#[async_trait]
pub trait Session {
    fn new(id: Id) -> SessionHandle<Self>;
    async fn add_peer(&self, id: peer::Id, peer: Arc<peer::Peer>) -> Result<()>;
    fn write_channel(&self) -> WriteStream;
}

pub enum SessionEvent {
    TrackPublished(MediaTrackRouterHandle),
}

pub struct LocalSession {
    id: Id,
    peers: Arc<Mutex<HashMap<peer::Id, Arc<peer::Peer>>>>,
    routers: Arc<Mutex<HashMap<String, MediaTrackRouterHandle>>>,
    tx: WriteStream,
}

#[async_trait]
impl Session for LocalSession {
    fn new(id: Id) -> SessionHandle<LocalSession> {
        let (tx, rx) = mpsc::channel(16);

        let handle = Arc::new(LocalSession {
            id: id,
            peers: Arc::new(Mutex::new(HashMap::new())),
            routers: Arc::new(Mutex::new(HashMap::new())),
            tx: tx,
        });

        tokio::spawn(enc!((handle) async move { LocalSession::event_loop(handle, rx).await } ));

        handle
    }

    fn write_channel(&self) -> WriteStream {
        self.tx.clone()
    }

    async fn add_peer(&self, id: peer::Id, peer: Arc<peer::Peer>) -> Result<()> {
        let mut peers = self.peers.lock().await;

        if peers.contains_key(&id) {
            error!("Peer id={} already exists", id);
            return Err(format_err!("Peer id={} already exists", id));
        }

        self.subscribe_peer_to_all_routers(&peer).await;
        peers.insert(id, peer);

        Ok(())
    }
}

impl LocalSession {
    async fn event_loop(session: SessionHandle<Self>, mut rx: mpsc::Receiver<SessionEvent>) {
        while let Some(evt) = rx.next().await {
            match evt {
                SessionEvent::TrackPublished(router) => session.add_router(router).await,
            }
        }
    }

    async fn add_router(&self, router: MediaTrackRouterHandle) {
        self.subscribe_all_peers_to_router(&router).await;

        let id = { router.lock().await.id.clone() };
        let mut routers = self.routers.lock().await;
        routers.insert(id, router);
    }

    async fn subscribe_all_peers_to_router(&self, router: &MediaTrackRouterHandle) {
        let mut peers = self.peers.lock().await;

        for (_, peer) in &mut *peers {
            let subscriber = { router.lock().await.add_subscriber().await };
            peer.add_media_track_subscriber(subscriber).await;
        }
    }

    async fn subscribe_peer_to_all_routers(&self, peer: &Arc<peer::Peer>) {
        let mut routers = self.routers.lock().await;

        for (_, router) in &mut *routers {
            let subscriber = { router.lock().await.add_subscriber().await };
            peer.add_media_track_subscriber(subscriber).await;
        }
    }
}
