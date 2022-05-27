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
use crate::sfu::session;

/// CascadeSession represents a node in the mesh for a given session id
/// All tracks from our clients are trunked via uplinks
/// and all tracks from the uplinks are trunked to the downlinks.
pub struct CascadeSession {
    id: session::Id,
    uplinks: Arc<Mutex<HashMap<peer::Id, Arc<peer::Peer>>>>,
    downlinks: Arc<Mutex<HashMap<peer::Id, Arc<peer::Peer>>>>,
    routers: Arc<Mutex<HashMap<String, MediaTrackRouterHandle>>>,
    tx: session::WriteStream,
}

#[async_trait]
impl session::Session for CascadeSession {
    fn new(id: session::Id) -> session::SessionHandle<CascadeSession> {
        let (tx, rx) = mpsc::channel(16);

        let handle = Arc::new(CascadeSession {
            id: id,
            uplinks: Arc::new(Mutex::new(HashMap::new())),
            downlinks: Arc::new(Mutex::new(HashMap::new())),
            routers: Arc::new(Mutex::new(HashMap::new())),
            tx: tx,
        });

        tokio::spawn(enc!((handle) async move { CascadeSession::event_loop(handle, rx).await } ));

        debug!("LocalSession(id={}) started", handle.id);
        handle
    }

    fn write_channel(&self) -> session::WriteStream {
        self.tx.clone()
    }

    async fn add_peer(&self, id: peer::Id, peer: Arc<peer::Peer>) -> Result<()> {
        let mut peers = self.downlinks.lock().await;

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
        let mut peers = self.downlinks.lock().await;

        if let Some(peer) = peers.remove(&id) {
            debug!("LocalSession(id={}) Removed Peer(id={})", self.id, id);
            peer.close().await;
        }

        Ok(())
    }
}

impl CascadeSession {
    async fn event_loop(
        session: session::SessionHandle<Self>,
        mut rx: mpsc::Receiver<session::SessionEvent>,
    ) {
        while let Some(evt) = rx.next().await {
            match evt {
                session::SessionEvent::TrackPublished(router) => session.add_router(router).await,
                session::SessionEvent::TrackRemoved(router_id) => {
                    session.remove_router(router_id).await
                }
            }
        }
    }

    /// Adds a peer that is another SFU signaled through the service mesh
    async fn add_uplink(&self, id: peer::Id, peer: Arc<peer::Peer>) -> Result<()> {
        let mut trunks = self.uplinks.lock().await;

        if trunks.contains_key(&id) {
            error!("Trunk id={} already exists", id);
            return Err(format_err!("Trunk id={} already exists", id));
        }

        self.subscribe_peer_to_all_routers(&peer).await;
        trunks.insert(id, peer);

        debug!("LocalSession(id={}) Added Trunk Uplink(id={})", self.id, id);

        Ok(())
    }

    async fn remove_uplink(&self, id: peer::Id) {}

    async fn add_router(&self, router: MediaTrackRouterHandle) {
        self.subscribe_all_peers_to_router(&router).await;

        let id = { router.lock().await.id.clone() };
        let mut routers = self.routers.lock().await;
        routers.insert(id, router);
    }

    async fn remove_router(&self, router_id: String) {
        let mut routers = self.routers.lock().await;
        let _router = routers.remove(&router_id);
    }

    async fn subscribe_all_peers_to_router(&self, router: &MediaTrackRouterHandle) {
        let mut peers = self.downlinks.lock().await;

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
