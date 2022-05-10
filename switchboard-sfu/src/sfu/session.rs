use super::peer;
use anyhow::{format_err, Result};
use async_mutex::Mutex;
use async_trait::async_trait;
use log::*;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// SessionID represents a collection of peers that can route tracks to eachother
pub type Id = Uuid;
pub type SessionHandle<T: Session> = Arc<Mutex<T>>;

#[async_trait]
pub trait Session {
    fn new(id: Id) -> SessionHandle<Self>;
    async fn add_peer(&self, id: peer::Id, peer: peer::Peer) -> Result<()>;
}

pub struct LocalSession {
    id: Id,
    peers: Arc<Mutex<HashMap<peer::Id, peer::Peer>>>,
}

#[async_trait]
impl Session for LocalSession {
    fn new(id: Id) -> SessionHandle<LocalSession> {
        Arc::new(Mutex::new(LocalSession {
            id: id,
            peers: Arc::new(Mutex::new(HashMap::new())),
        }))
    }
    async fn add_peer(&self, id: peer::Id, peer: peer::Peer) -> Result<()> {
        let mut peers = self.peers.lock().await;

        if peers.contains_key(&id) {
            error!("Peer id={} already exists", id);
            return Err(format_err!("Peer id={} already exists", id));
        }

        peers.insert(id, peer);
        Ok(())
    }
}
