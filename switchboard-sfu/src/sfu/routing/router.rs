use anyhow::{format_err, Result};
use async_mutex::Mutex;
use enclose::enc;
use futures::StreamExt;
use futures_channel::{mpsc, oneshot};
use log::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use webrtc::rtp;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::track::track_remote::TrackRemote;

use super::*;
use crate::sfu::peer;

pub type Id = String;
pub type MediaTrackRouterHandle = Arc<Mutex<MediaTrackRouter>>;

/// Packet contains an rtp::packet::Packet and associated information
#[derive(Clone)]
pub struct Packet {
    //keyframe: bool,
    pub layer: Arc<Layer>,
    pub rtp: rtp::packet::Packet,
}

/// MediaTrackRouter receives RTP from a TrackRemote and can generate MediaTrackSubscribers to
/// write the track to multiple other peer connections
pub struct MediaTrackRouter {
    pub id: Id,
    track_remotes: HashMap<Layer, Arc<TrackRemote>>,

    event_tx: mpsc::Sender<MediaTrackSubscriberEvent>,
    packet_sender: broadcast::Sender<Packet>,
}

impl MediaTrackRouter {
    pub async fn new(
        ssrc: u32,
        tid: String,
        rtcp_writer: peer::RtcpWriter,
    ) -> MediaTrackRouterHandle {
        let (pkt_tx, _pkt_rx) = broadcast::channel(512);
        let (evt_tx, evt_rx) = mpsc::channel(32);

        tokio::spawn(
            async move { MediaTrackRouter::rtcp_event_loop(ssrc, evt_rx, rtcp_writer).await },
        );

        Arc::new(Mutex::new(MediaTrackRouter {
            id: tid,
            track_remotes: HashMap::new(),
            packet_sender: pkt_tx,
            event_tx: evt_tx,
        }))
    }

    /// Add's a track remote as a layer on this router
    pub async fn add_layer(
        &mut self,
        track_remote: Arc<TrackRemote>,
        _rtp_receiver: Arc<RTCRtpReceiver>,
    ) -> oneshot::Receiver<bool> {
        let pkt_tx = self.packet_sender.clone();

        let (closed_tx, closed_rx) = oneshot::channel();
        tokio::spawn(enc!((pkt_tx, track_remote) async move {
            MediaTrackRouter::rtp_event_loop(track_remote, pkt_tx).await;
            let _ = closed_tx.send(true);
        }));

        let layer = Layer::from(track_remote.rid());
        self.track_remotes.insert(layer, track_remote);

        closed_rx
    }

    pub async fn add_subscriber(&self) -> Result<MediaTrackSubscriber> {
        let track_remote = match self.track_remotes.values().next() {
            Some(track) => track,
            None => {
                return Err(format_err!(
                    "MediaTrackRouter could not add subscriber: no remote tracks have been added to this router"
                ));
            }
        };

        trace!("MediaTrackRouter adding new subscriber");
        let event_tx = self.event_tx.clone();
        Ok(
            MediaTrackSubscriber::new(&track_remote, self.packet_sender.subscribe(), event_tx)
                .await,
        )
    }

    async fn rtcp_event_loop(
        media_ssrc: u32,
        mut event_rx: mpsc::Receiver<MediaTrackSubscriberEvent>,
        mut rtcp_writer: peer::RtcpWriter,
    ) {
        while let Some(event) = event_rx.next().await {
            match event {
                MediaTrackSubscriberEvent::PictureLossIndication => {
                    trace!("MediaTrackRouter forwarding PLI from MediaTrackSubscriber");
                    rtcp_writer
                        .try_send(Box::new(PictureLossIndication {
                            sender_ssrc: 0,
                            media_ssrc,
                        }))
                        .expect("Couldn't forward PLI to MediaTrackRouter");
                }
            }
        }
        debug!("MediaTrackRouter RTCP Event Loop finished");
    }

    // Process RTCP & RTP packets for this track
    pub async fn rtp_event_loop(track: Arc<TrackRemote>, packet_sender: broadcast::Sender<Packet>) {
        debug!(
            "MediaTrackRouter has started, of type {} rid={} : {}",
            track.payload_type(),
            track.rid(),
            track.codec().await.capability.mime_type
        );

        let layer = Arc::new(match track.rid() {
            "" => Layer::Unicast,
            layer => Layer::Rid(layer.to_owned()),
        });

        let mut last_timestamp = 0;
        while let Ok((mut rtp, _attr)) = track.read_rtp().await {
            // Change the timestamp to only be the delta
            let old_timestamp = rtp.header.timestamp;
            if last_timestamp == 0 {
                rtp.header.timestamp = 0
            } else {
                rtp.header.timestamp -= last_timestamp;
            }
            last_timestamp = old_timestamp;

            trace!(
                "MediaTrackRouter received RTP :layer={:?} ssrc={} seq={} timestamp={}",
                layer,
                rtp.header.ssrc,
                rtp.header.sequence_number,
                rtp.header.timestamp
            );

            // Send packet to broadcast channel
            if packet_sender.receiver_count() > 0 {
                if let Err(e) = packet_sender.send(Packet {
                    layer: layer.clone(),
                    rtp,
                }) {
                    error!("MediaTrackRouter failed to broadcast RTP: {}", e);
                }
            } else {
                trace!("MediaTrackRouter has no subscribers");
            }
        }

        debug!(
            "MediaTrackRouter has ended, of type {}: {}",
            track.payload_type(),
            track.codec().await.capability.mime_type
        );
    }
}
