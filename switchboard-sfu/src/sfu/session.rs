use anyhow::{format_err, Result};
use async_mutex::Mutex;
use async_trait::async_trait;
use enclose::enc;
use futures::StreamExt;
use futures_channel::mpsc;
use log::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

use crate::sfu::peer;
use crate::sfu::routing::MediaTrackRouterHandle;
use crate::signal::signal;

// SessionID represents a collection of peers that can route tracks to eachother
pub type Id = String;
pub type SessionHandle<T> = Arc<T>;

pub type ReadStream = mpsc::Receiver<SessionEvent>;
pub type WriteStream = mpsc::Sender<SessionEvent>;

#[async_trait]
pub trait Session {
    fn new(id: Id) -> SessionHandle<Self>;
    fn id(&self) -> Id;
    async fn active(&self) -> bool;
    fn write_channel(&self) -> WriteStream;

    async fn add_peer(&self, id: peer::Id, peer: Arc<peer::Peer>) -> Result<()>;
    async fn remove_peer(&self, id: peer::Id) -> Result<()>;

    async fn presence_set(&self, id: peer::Id, meta: serde_json::Value);
}

pub enum SessionEvent {
    TrackPublished(MediaTrackRouterHandle),
    TrackRemoved(String),
}

pub struct LocalSession {
    pub id: Id,
    peers: Arc<Mutex<HashMap<peer::Id, Arc<peer::Peer>>>>,
    routers: Arc<Mutex<HashMap<String, MediaTrackRouterHandle>>>,
    tx: WriteStream,

    presence_meta: Arc<Mutex<HashMap<peer::Id, serde_json::Value>>>,
    presence_revision: AtomicU64,
}

#[async_trait]
impl Session for LocalSession {
    fn new(id: Id) -> SessionHandle<LocalSession> {
        let (tx, rx) = mpsc::channel(16);

        let handle = Arc::new(LocalSession {
            id,
            peers: Arc::new(Mutex::new(HashMap::new())),
            routers: Arc::new(Mutex::new(HashMap::new())),
            tx,

            presence_meta: Arc::new(Mutex::new(HashMap::new())),
            presence_revision: AtomicU64::new(0),
        });

        tokio::spawn(enc!((handle) async move { LocalSession::event_loop(handle, rx).await } ));

        debug!("LocalSession(id={}) started", handle.id);
        handle
    }

    fn id(&self) -> Id {
        self.id.clone()
    }

    async fn active(&self) -> bool {
        let peers = self.peers.lock().await;
        peers.len() > 0
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

        debug!("LocalSession(id={}) Added Peer(id={})", self.id, id);

        Ok(())
    }

    async fn remove_peer(&self, id: peer::Id) -> Result<()> {
        let mut peers = self.peers.lock().await;

        if let Some(peer) = peers.remove(&id) {
            debug!("LocalSession(id={}) Removed Peer(id={})", self.id, id);
            peer.close().await;
        }

        Ok(())
    }

    async fn presence_set(&self, id: peer::Id, meta: serde_json::Value) {
        let mut presence = self.presence_meta.lock().await;
        presence.insert(id, meta);

        let mut rev: u64 = self.presence_revision.load(Ordering::SeqCst);
        rev += 1;
        self.presence_revision.store(rev, Ordering::SeqCst);

        let p = signal::Presence {
            revision: rev,
            meta: serde_json::to_value(&*presence).unwrap(),
        };

        let peers = self.peers.lock().await;

        for (_, peer) in &*peers {
            peer.signal_tx
                .unbounded_send(Ok(signal::Event::Presence(p.clone())))
                .ok();
        }
    }
}

impl LocalSession {
    async fn event_loop(session: SessionHandle<Self>, mut rx: mpsc::Receiver<SessionEvent>) {
        while let Some(evt) = rx.next().await {
            match evt {
                SessionEvent::TrackPublished(router) => session.add_router(router).await,
                SessionEvent::TrackRemoved(router_id) => session.remove_router(router_id).await,
            }
        }
    }

    async fn add_router(&self, router: MediaTrackRouterHandle) {
        self.subscribe_all_peers_to_router(&router).await;

        let id = { router.lock().await.id.clone() };
        let mut routers = self.routers.lock().await;
        routers.insert(id, router);
    }

    async fn remove_router(&self, router_id: String) {
        let _ = self.peers.lock().await;
        let mut routers = self.routers.lock().await;
        let _router = routers.remove(&router_id);
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
