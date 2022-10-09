use async_mutex::Mutex;
use enclose::enc;
use futures::StreamExt;
use futures_channel::{mpsc, oneshot};
use log::*;
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

/// MediaTrackRouter receives RTP from a TrackRemote and can generate MediaTrackSubscribers to
/// write the track to multiple other peer connections
pub struct MediaTrackRouter {
    pub id: Id,
    track_remote: Arc<TrackRemote>,

    event_tx: mpsc::Sender<MediaTrackSubscriberEvent>,
    packet_sender: broadcast::Sender<rtp::packet::Packet>,
    _rtp_receiver: Arc<RTCRtpReceiver>,
}

impl MediaTrackRouter {
    pub async fn new(
        track_remote: Arc<TrackRemote>,
        rtp_receiver: Arc<RTCRtpReceiver>,
        rtcp_writer: peer::RtcpWriter,
    ) -> (MediaTrackRouterHandle, oneshot::Receiver<bool>) {
        let (pkt_tx, _pkt_rx) = broadcast::channel(512);
        let (evt_tx, evt_rx) = mpsc::channel(32);

        let media_ssrc = track_remote.ssrc();
        tokio::spawn(async move {
            MediaTrackRouter::rtcp_event_loop(media_ssrc, evt_rx, rtcp_writer).await
        });

        let (closed_tx, closed_rx) = oneshot::channel();
        tokio::spawn(enc!((pkt_tx, track_remote) async move {
            MediaTrackRouter::rtp_event_loop(track_remote, pkt_tx).await;
            let _ = closed_tx.send(true);
        }));

        (
            Arc::new(Mutex::new(MediaTrackRouter {
                id: track_remote.id().await,
                track_remote,
                packet_sender: pkt_tx,
                _rtp_receiver: rtp_receiver,
                event_tx: evt_tx,
            })),
            closed_rx,
        )
    }

    pub async fn add_subscriber(&self) -> MediaTrackSubscriber {
        trace!("MediaTrackRouter adding new subscriber");

        let event_tx = self.event_tx.clone();
        MediaTrackSubscriber::new(&self.track_remote, self.packet_sender.subscribe(), event_tx)
            .await
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
    pub async fn rtp_event_loop(
        track: Arc<TrackRemote>,
        packet_sender: broadcast::Sender<rtp::packet::Packet>,
    ) {
        debug!(
            "MediaTrackRouter has started, of type {}: {}",
            track.payload_type(),
            track.codec().await.capability.mime_type
        );

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
                "MediaTrackRouter received RTP ssrc={} seq={} timestamp={}",
                rtp.header.ssrc,
                rtp.header.sequence_number,
                rtp.header.timestamp
            );

            // Send packet to broadcast channel
            if packet_sender.receiver_count() > 0 {
                if let Err(e) = packet_sender.send(rtp) {
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
