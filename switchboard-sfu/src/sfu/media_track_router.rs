use anyhow::Result;
use log::*;
use std::sync::Arc;
use tokio::sync::broadcast;
use webrtc::rtp;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::track::track_remote::TrackRemote;

pub struct MediaTrackRouter {
    track_remote: Arc<TrackRemote>,
    rtp_receiver: RTCRtpReceiver,

    packet_sender: broadcast::Sender<rtp::packet::Packet>,
    packet_receiver: broadcast::Receiver<rtp::packet::Packet>,
}

impl MediaTrackRouter {
    pub fn new(track_remote: TrackRemote, rtp_receiver: RTCRtpReceiver) -> MediaTrackRouter {
        let (pkt_tx, pkt_rx) = broadcast::channel(512);
        MediaTrackRouter {
            track_remote: Arc::new(track_remote),
            rtp_receiver: rtp_receiver,

            packet_sender: pkt_tx,
            packet_receiver: pkt_rx,
        }
    }

    pub fn packet_receiver(&self) -> broadcast::Receiver<rtp::packet::Packet> {
        self.packet_sender.subscribe()
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

                // Send packet to broadcast channel
                if let Err(e) = packet_sender.send(rtp) {
                    error!("MediaTrackRouter failed to broadcast RTP: {}", e);
                }
            }

            println!(
                "MediaTrackRouter has ended, of type {}: {}",
                track.payload_type(),
                track.codec().await.capability.mime_type
            );
        });

        //let media_ssrc = self.track_remote.ssrc();
        //tokio::spawn(async move {
        //    let mut result = Result::<usize>::Ok(0);
        //    while result.is_ok() {
        //        let timeout = tokio::time::sleep(Duration::from_secs(3));
        //        tokio::pin!(timeout);

        //        tokio::select! {
        //            _ = timeout.as_mut() =>{
        //                if let Some(pub_pc) = pub_pc.upgrade(){
        //                    result = pub_pc.write_rtcp(&[Box::new(PictureLossIndication{
        //                        sender_ssrc: 0,
        //                        media_ssrc,
        //                    })]).await.map_err(Into::into);
        //                }else{
        //                    break;
        //                }
        //            }
        //        };
        //    }
        //});
    }
}
