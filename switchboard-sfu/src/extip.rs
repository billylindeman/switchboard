use anyhow::{format_err, Result};
use log::*;
use std::sync::Arc;
use tokio::sync::mpsc;

use webrtc::ice::agent::agent_config::AgentConfig;
use webrtc::ice::agent::Agent;
use webrtc::ice::candidate::*;
use webrtc::ice::network_type::*;
use webrtc::ice::url::Url;

pub async fn resolve_external_ip_maps() -> Result<Vec<String>> {
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
                        tx_clone
                            .send((c.address(), c.candidate_type()))
                            .await
                            .unwrap();
                    }
                })
            },
        ))
        .await;

    ice_agent.gather_candidates().await?;

    let mut hosts = vec![];
    while let Some((addr, t)) = rx.recv().await {
        match t {
            CandidateType::Host => {
                debug!("Resolved host ip {:?}", addr);
                hosts.push(addr);
            }
            CandidateType::ServerReflexive => {
                debug!("Resolved ext ip {:?}", addr);
                let mut maps = vec![];

                for h in hosts {
                    maps.push(format!("{}/{}", addr, h));
                }

                return Ok(maps);
            }
            _ => {}
        }
    }

    Err(format_err!("could not resolve external ip"))
}
