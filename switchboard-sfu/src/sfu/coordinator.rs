use super::session;
use async_mutex::Mutex;
use async_trait::async_trait;
use log::*;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait Coordinator<S: session::Session> {
    async fn get_or_create_session(&self, id: session::Id) -> session::SessionHandle<S>;
}

pub struct LocalCoordinator<S: session::Session> {
    pub sessions: Arc<Mutex<HashMap<session::Id, session::SessionHandle<S>>>>,
}

impl<S: session::Session> LocalCoordinator<S> {
    pub fn new() -> LocalCoordinator<S> {
        return LocalCoordinator {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        };
    }
}

#[async_trait]
impl<S: session::Session + Send> Coordinator<S> for LocalCoordinator<S> {
    async fn get_or_create_session(&self, id: session::Id) -> session::SessionHandle<S> {
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get(&id) {
            trace!("get_or_create_session id={} returning existing session", id);
            return session.clone();
        }

        debug!("LocalCoodinator starting new session id={}", id);
        let session = S::new(id);
        sessions.insert(id, session.clone());
        session
    }
}
