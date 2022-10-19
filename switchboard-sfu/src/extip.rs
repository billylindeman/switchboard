use anyhow::{format_err, Result};
use log::*;
use std::sync::Arc;
use tokio::sync::mpsc;

use webrtc::ice::agent::agent_config::AgentConfig;
use webrtc::ice::agent::Agent;
use webrtc::ice::candidate::*;
use webrtc::ice::network_type::*;
use webrtc::ice::url::Url;

pub async fn resolve_external_ip() -> Result<String> {
    let ice_agent = Arc::new(
        Agent::new(AgentConfig {
            urls: vec![Url::parse_url("stun:stun.l.google.com:19302")?],
            network_types: vec![NetworkType::Udp4],
            ..Default::default()
        })
        .await?,
    );

    let (tx, mut rx) = mpsc::channel(16);

    ice_agent
        .on_candidate(Box::new(
            move |c: Option<Arc<dyn Candidate + Send + Sync>>| {
                let tx_clone = tx.clone();
                Box::pin(async move {
                    if let Some(c) = c {
                        debug!(
                            "Gathered External Candidate: {:?} {:?}",
                            c.address(),
                            c.candidate_type()
                        );

                        if c.candidate_type() == CandidateType::ServerReflexive {
                            tx_clone.send(c.address()).await.unwrap();
                        }
                    }
                })
            },
        ))
        .await;

    ice_agent.gather_candidates().await?;

    if let Some(extip) = rx.recv().await {
        debug!("Resolved external ip {}", extip);
        return Ok(extip);
    }

    Err(format_err!("could not resolve external ip"))
}
