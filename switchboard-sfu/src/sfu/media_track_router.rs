use anyhow::Result;
use futures::{Stream, StreamExt};
use futures_channel::mpsc;
use log::*;
use std::sync::Arc;
use tokio::sync::broadcast;
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

use super::peer;

pub struct MediaTrackRouter {
    track_remote: Arc<TrackRemote>,
    packet_sender: broadcast::Sender<rtp::packet::Packet>,

    rtcp_writer: Arc<peer::RtcpWriter>,
    event_rx: Arc<mpsc::Receiver<MediaTrackSubscriberEvent>>,
    event_tx: mpsc::Sender<MediaTrackSubscriberEvent>,

    _packet_receiver: broadcast::Receiver<rtp::packet::Packet>,
    _rtp_receiver: Arc<RTCRtpReceiver>,
}

impl MediaTrackRouter {
    pub fn new(
        track_remote: Arc<TrackRemote>,
        rtp_receiver: Arc<RTCRtpReceiver>,
        rtcp_writer: peer::RtcpWriter,
    ) -> MediaTrackRouter {
        let (pkt_tx, pkt_rx) = broadcast::channel(512);

        let (evt_tx, evt_rx) = mpsc::channel(32);

        MediaTrackRouter {
            track_remote: track_remote,
            _rtp_receiver: rtp_receiver,
            rtcp_writer: Arc::new(rtcp_writer),
            event_rx: Arc::new(evt_rx),
            event_tx: evt_tx,

            packet_sender: pkt_tx,
            _packet_receiver: pkt_rx,
        }
    }

    pub async fn add_subscriber(&self) -> MediaTrackSubscriber {
        trace!("MediaTrackRouter adding new subscriber");

        let event_tx = self.event_tx.clone();
        MediaTrackSubscriber::new(&self.track_remote, self.packet_sender.subscribe(), event_tx)
            .await
    }

    // Process RTCP & RTP packets for this track
    pub async fn event_loop(&self) {
        let track = self.track_remote.clone();
        let packet_sender = self.packet_sender.clone();
        tokio::spawn(async move {
            debug!(
                "MediaTrackRouter has started, of type {}: {}",
                track.payload_type(),
                track.codec().await.capability.mime_type
            );

            let mut last_timestamp = 0;
            while let Ok((mut rtp, _)) = track.read_rtp().await {
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

            println!(
                "MediaTrackRouter has ended, of type {}: {}",
                track.payload_type(),
                track.codec().await.capability.mime_type
            );
        });

        let media_ssrc = self.track_remote.ssrc();
        let mut event_rx = self.event_rx.clone();
        let mut rtcp_writer = self.rtcp_writer.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.next().await {
                match event {
                    PictureLossIndication => {
                        rtcp_writer.try_send(Box::new(PictureLossIndication {
                            sender_ssrc: 0,
                            media_ssrc,
                        }));
                    }
                }
            }
        });
    }
}

enum MediaTrackSubscriberEvent {
    PictureLossIndication,
}

pub struct MediaTrackSubscriber {
    track: Arc<TrackLocalStaticRTP>,
    pkt_receiver: broadcast::Receiver<rtp::packet::Packet>,
    evt_sender: mpsc::Sender<MediaTrackSubscriberEvent>,
}

impl MediaTrackSubscriber {
    pub async fn new(
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

    pub async fn add_to_peer_connection(&self, peer_connection: &RTCPeerConnection) -> Result<()> {
        debug!("MediaTrackSubscriber added to peer_connection");

        // Add this newly created track to the PeerConnection
        let rtp_sender = peer_connection
            .add_track(Arc::clone(&self.track) as Arc<dyn TrackLocal + Send + Sync>)
            .await?;

        let evt_sender = self.evt_sender.clone();

        // Read incoming RTCP packets
        // Before these packets are returned they are processed by interceptors. For things
        // like NACK this needs to be called
        tokio::spawn(async move { MediaTrackSubscriber::rtcp_event_loop(rtp_sender, evt_sender) });

        Ok(())
    }

    pub async fn event_loop(&mut self) {
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
                    return;
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

    pub async fn rtcp_event_loop(
        rtp_sender: Arc<RTCRtpSender>,
        mut evt_sender: mpsc::Sender<MediaTrackSubscriberEvent>,
    ) {
        use rtcp::header::{PacketType, FORMAT_PLI};

        while let Ok((rtcp_packets, _)) = rtp_sender.read_rtcp().await {
            for rtcp in rtcp_packets.into_iter() {
                trace!("got rtcp {:#?}", rtcp);

                let header = rtcp.header();
                match header.packet_type {
                    PacketType::ReceiverReport => {
                        let rr = &rtcp
                            .as_any()
                            .downcast_ref::<rtcp::receiver_report::ReceiverReport>()
                            .unwrap();

                        trace!("got receiver report: {:#?}", rr);
                    }
                    PacketType::PayloadSpecificFeedback => match header.count {
                        FORMAT_PLI => {
                            let pli = &rtcp
                                    .as_any()
                                    .downcast_ref::<rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication>()
                                    .unwrap();
                            trace!("got pli {:#?}", pli);

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
    }
}
