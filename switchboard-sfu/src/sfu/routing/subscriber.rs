use anyhow::Result;
use async_mutex::Mutex;
use enclose::enc;
use futures::{Stream, StreamExt};
use futures_channel::{mpsc, oneshot};
use log::*;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtcp;
use webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use webrtc::rtp;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::rtp_transceiver::rtp_sender::RTCRtpSender;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};
use webrtc::track::track_remote::TrackRemote;
use webrtc::Error;

use crate::sfu::peer;

pub(super) enum MediaTrackSubscriberEvent {
    PictureLossIndication,
}

pub struct MediaTrackSubscriber {
    track: Arc<TrackLocalStaticRTP>,
    pkt_receiver: broadcast::Receiver<rtp::packet::Packet>,
    evt_sender: mpsc::Sender<MediaTrackSubscriberEvent>,
}

impl MediaTrackSubscriber {
    pub(super) async fn new(
        remote: &TrackRemote,
        pkt_receiver: broadcast::Receiver<rtp::packet::Packet>,
        evt_sender: mpsc::Sender<MediaTrackSubscriberEvent>,
    ) -> MediaTrackSubscriber {
        let output_track = Arc::new(TrackLocalStaticRTP::new(
            remote.codec().await.capability,
            remote.id().await,
            remote.stream_id().await,
        ));

        debug!(
            "MediaTrackSubscriber created track={} stream={}",
            output_track.id(),
            output_track.stream_id()
        );
        MediaTrackSubscriber {
            track: output_track,
            pkt_receiver: pkt_receiver,
            evt_sender: evt_sender,
        }
    }

    pub async fn add_to_peer_connection(
        &self,
        peer_connection: &RTCPeerConnection,
    ) -> Result<Arc<RTCRtpSender>> {
        debug!("MediaTrackSubscriber added to peer_connection");

        // Add this newly created track to the PeerConnection
        let rtp_sender = peer_connection
            .add_track(Arc::clone(&self.track) as Arc<dyn TrackLocal + Send + Sync>)
            .await?;

        let evt_sender = self.evt_sender.clone();

        // Read incoming RTCP packets
        // Before these packets are returned they are processed by interceptors. For things
        // like NACK this needs to be called
        tokio::spawn(
            enc!((rtp_sender) async move { MediaTrackSubscriber::rtcp_event_loop(rtp_sender, evt_sender).await }),
        );

        Ok(rtp_sender)
    }

    pub async fn rtp_event_loop(&mut self) {
        debug!(
            "MediaTrackSubscriber starting track={} stream={}",
            self.track.id(),
            self.track.stream_id()
        );

        // Asynchronously take all packets in the channel and write them out to our
        // track
        let mut curr_timestamp = 0;
        let mut i = 0;

        while let Ok(mut packet) = self.pkt_receiver.recv().await {
            // Timestamp on the packet is really a diff, so add it to current
            curr_timestamp += packet.header.timestamp;
            packet.header.timestamp = curr_timestamp;
            // Keep an increasing sequence number
            packet.header.sequence_number = i;

            trace!(
                "MediaTrackSubscriber wrote RTP ssrc={} seq={} timestamp={}",
                packet.header.ssrc,
                packet.header.sequence_number,
                packet.header.timestamp
            );

            // Write out the packet, ignoring closed pipe if nobody is listening
            if let Err(err) = self.track.write_rtp(&packet).await {
                if Error::ErrClosedPipe == err {
                    // The peerConnection has been closed.
                    debug!("MediaTrackSubscriber write_rtp ErrClosedPipe");
                    break;
                } else {
                    error!("MediaTrackSubscriber failed {}", err);
                }
            }
            i += 1;
        }

        debug!(
            "MediaTrackSubscriber stopped track={} stream={}",
            self.track.id(),
            self.track.stream_id()
        );
    }

    async fn rtcp_event_loop(
        rtp_sender: Arc<RTCRtpSender>,
        mut evt_sender: mpsc::Sender<MediaTrackSubscriberEvent>,
    ) {
        use rtcp::header::{PacketType, FORMAT_PLI};

        debug!("MediaTrackSubscriber RTCP ReadLoop starting");

        while let Ok((rtcp_packets, attr)) = rtp_sender.read_rtcp().await {
            for rtcp in rtcp_packets.into_iter() {
                trace!(
                    "MediaTrackSubscriber RTCP ReadLoop => rtcp={:#?} attr={:#?}",
                    rtcp,
                    attr
                );

                let header = rtcp.header();
                match header.packet_type {
                    PacketType::ReceiverReport => {
                        let rr = &rtcp
                            .as_any()
                            .downcast_ref::<rtcp::receiver_report::ReceiverReport>()
                            .unwrap();
                    }
                    PacketType::PayloadSpecificFeedback => match header.count {
                        FORMAT_PLI => {
                            let pli = &rtcp
                                    .as_any()
                                    .downcast_ref::<rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication>()
                                    .unwrap();
                            evt_sender
                                .try_send(MediaTrackSubscriberEvent::PictureLossIndication)
                                .unwrap();
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        debug!("MediaTrackSubscriber RTCP ReadLoop stopped");
    }
}
