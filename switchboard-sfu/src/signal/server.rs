use futures_util::{SinkExt, StreamExt, TryStreamExt};
use log::*;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

use super::*;

use crate::sfu::coordinator::{Coordinator, LocalCoordinator};
use crate::sfu::peer;
use crate::sfu::session;
use crate::sfu::session::{LocalSession, Session};

/// Spawns a tokio tcp server
pub async fn run_server(addr: &str) {
    let coordinator: Arc<LocalCoordinator<LocalSession>> = LocalCoordinator::new();

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection::<
            LocalCoordinator<LocalSession>,
            LocalSession,
        >(coordinator.clone(), stream));
    }
}

/// Handles a websocket connection for a given Coordinator<S>
async fn accept_connection<C, S>(coordinator: Arc<C>, stream: TcpStream)
where
    C: Coordinator<S>,
    S: Session,
{
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    info!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (rpc_rx, rpc_tx) = jsonrpc::handle_messages(ws_stream).await;
    let (sig_rx, mut sig_tx) = signal::handle_messages(rpc_rx, rpc_tx).await;

    event_loop(coordinator, sig_rx, sig_tx.clone()).await;

    error!("event loop closed");
    sig_tx.close().await.expect("closed signal tx");

    info!("client disconnected");
}

/// Event loop for each signal connection
pub async fn event_loop<C, S>(
    coordinator: Arc<C>,
    mut rx: signal::ReadStream,
    tx: signal::WriteStream,
) where
    C: Coordinator<S>,
    S: Session,
{
    let mut peer: Option<Arc<peer::Peer>> = None;
    let mut joined_session: Option<session::SessionHandle<S>> = None;

    while let Some(Ok(evt)) = rx.next().await {
        match evt {
            signal::Event::JoinRequest(res, join) => {
                info!("got join request: {:#?}", join);

                let session = coordinator.get_or_create_session(join.sid).await;

                let p = peer::Peer::new(tx.clone(), session.write_channel(), None)
                    .await
                    .expect("Error creating peer");

                let answer = p.publisher_get_answer_for_offer(join.offer).await;
                if let Err(err) = &answer {
                    error!("Error with join offer {}", err);
                };

                info!("answer created ");
                session
                    .add_peer(p.id, p.clone())
                    .await
                    .expect("error adding peer to session");

                peer = Some(p.clone());
                joined_session = Some(session);

                res.send(answer.unwrap()).expect("error sending response");
            }

            signal::Event::TrickleIce(trickle) => match &peer {
                Some(peer) => {
                    info!("trickle ice: {:#?}", trickle);
                    peer.trickle_ice_candidate(trickle.target, trickle.candidate.into())
                        .await
                        .expect("error adding trickle candidate");
                }
                None => {
                    error!("peer has not joined session yet");
                }
            },

            signal::Event::PublisherOffer(res, offer) => match &peer {
                Some(peer) => {
                    info!("publisher made offer");

                    let answer = peer
                        .publisher_get_answer_for_offer(offer.desc)
                        .await
                        .expect("publisher error setting remote description");

                    res.send(answer).expect("error sending answer");
                }
                None => {
                    error!("peer has not joined session yet");
                }
            },

            signal::Event::SubscriberAnswer(answer) => match &peer {
                Some(peer) => {
                    info!("subscriber got answer");
                    peer.subscriber_set_answer(answer.desc)
                        .await
                        .expect("subscriber error setting remote description");
                }
                None => {
                    error!("peer has not joined session yet");
                }
            },
            _ => {}
        }
    }

    info!("signal event loop finished");

    if let Some(session) = joined_session {
        if let Some(peer) = peer {
            session
                .remove_peer(peer.id)
                .await
                .expect("error removing peer");
        }
    }
}
