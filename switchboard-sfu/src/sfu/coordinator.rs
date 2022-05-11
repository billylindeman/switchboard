use super::session;
use async_mutex::Mutex;
use async_trait::async_trait;
use log::*;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait Coordinator<S: session::Session> {
    fn new() -> Arc<Self>;
    async fn get_or_create_session(&self, id: session::Id) -> session::SessionHandle<S>;
}

pub struct LocalCoordinator<S: session::Session> {
    pub sessions: Arc<Mutex<HashMap<session::Id, session::SessionHandle<S>>>>,
}

#[async_trait]
impl<S: session::Session + Send + Sync> Coordinator<S> for LocalCoordinator<S> {
    fn new() -> Arc<LocalCoordinator<S>> {
        Arc::new(LocalCoordinator {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    async fn get_or_create_session(&self, id: session::Id) -> session::SessionHandle<S> {
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get(&id) {
            trace!("get_or_create_session id={} returning existing session", id);
            return session.clone();
        }

        debug!("LocalCoodinator starting new session id={}", id);
        let session = S::new(id.clone());
        sessions.insert(id.clone(), session.clone());
        session
    }
}
