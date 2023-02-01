use webrtc::error::Result;
use webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTCRtpHeaderExtensionCapability, RTPCodecType,
};

use webrtc::api::media_engine::*;
use webrtc::rtp_transceiver::RTCPFeedback;

const EXT_URI_SDES_MID: &str = "urn:ietf:params:rtp-hdrext:sdes:mid";
const EXT_URI_SDES_RTP_SID: &str = "urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id";
const EXT_URI_SDES_REP_SID: &str = "urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id";
const EXT_URI_AUDIO_LEVEL: &str = "urn:ietf:params:rtp-hdrext:ssrc-audio-level";

pub fn register_default_codecs(media_engine: &mut MediaEngine) -> Result<()> {
    // Default Audio Codecs
    for codec in vec![
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                clock_rate: 48000,
                channels: 2,
                sdp_fmtp_line: "minptime=10;useinbandfec=1".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 111,
            ..Default::default()
        },
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_G722.to_owned(),
                clock_rate: 8000,
                channels: 0,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 9,
            ..Default::default()
        },
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_PCMU.to_owned(),
                clock_rate: 8000,
                channels: 0,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 0,
            ..Default::default()
        },
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_PCMA.to_owned(),
                clock_rate: 8000,
                channels: 0,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 8,
            ..Default::default()
        },
    ] {
        media_engine.register_codec(codec, RTPCodecType::Audio)?;
    }

    let video_rtcp_feedback = vec![
        RTCPFeedback {
            typ: "goog-remb".to_owned(),
            parameter: "".to_owned(),
        },
        RTCPFeedback {
            typ: "ccm".to_owned(),
            parameter: "fir".to_owned(),
        },
        RTCPFeedback {
            typ: "nack".to_owned(),
            parameter: "".to_owned(),
        },
        RTCPFeedback {
            typ: "nack".to_owned(),
            parameter: "pli".to_owned(),
        },
    ];
    for codec in vec![
        //RTCRtpCodecParameters {
        //    capability: RTCRtpCodecCapability {
        //        mime_type: MIME_TYPE_AV1.to_owned(),
        //        clock_rate: 90000,
        //        channels: 0,
        //        sdp_fmtp_line: "profile-id=0".to_owned(),
        //        rtcp_feedback: video_rtcp_feedback.clone(),
        //    },
        //    payload_type: 41,
        //    ..Default::default()
        //},
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP9.to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "profile-id=0".to_owned(),
                rtcp_feedback: video_rtcp_feedback.clone(),
            },
            payload_type: 98,
            ..Default::default()
        },
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP8.to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: video_rtcp_feedback.clone(),
            },
            payload_type: 96,
            ..Default::default()
        },
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: "video/rtx".to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "apt=96".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 97,
            ..Default::default()
        },
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: "video/rtx".to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "apt=98".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 99,
            ..Default::default()
        },
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP9.to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "profile-id=1".to_owned(),
                rtcp_feedback: video_rtcp_feedback.clone(),
            },
            payload_type: 100,
            ..Default::default()
        },
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: "video/rtx".to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "apt=100".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 101,
            ..Default::default()
        },
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_H264.to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line:
                    "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=F4001f"
                        .to_owned(),
                rtcp_feedback: video_rtcp_feedback.clone(),
            },
            payload_type: 123,
            ..Default::default()
        },
        // RTCRtpCodecParameters {
        //     capability: RTCRtpCodecCapability {
        //         mime_type: MIME_TYPE_H264.to_owned(),
        //         clock_rate: 90000,
        //         channels: 0,
        //         sdp_fmtp_line:
        //             "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42001f"
        //                 .to_owned(),
        //         rtcp_feedback: video_rtcp_feedback.clone(),
        //     },
        //     payload_type: 102,
        //     ..Default::default()
        // },
        // RTCRtpCodecParameters {
        //     capability: RTCRtpCodecCapability {
        //         mime_type: "video/rtx".to_owned(),
        //         clock_rate: 90000,
        //         channels: 0,
        //         sdp_fmtp_line: "apt=102".to_owned(),
        //         rtcp_feedback: vec![],
        //     },
        //     payload_type: 121,
        //     ..Default::default()
        // },
        // RTCRtpCodecParameters {
        //     capability: RTCRtpCodecCapability {
        //         mime_type: MIME_TYPE_H264.to_owned(),
        //         clock_rate: 90000,
        //         channels: 0,
        //         sdp_fmtp_line:
        //             "level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=42001f"
        //                 .to_owned(),
        //         rtcp_feedback: video_rtcp_feedback.clone(),
        //     },
        //     payload_type: 127,
        //     ..Default::default()
        // },
        // RTCRtpCodecParameters {
        //     capability: RTCRtpCodecCapability {
        //         mime_type: "video/rtx".to_owned(),
        //         clock_rate: 90000,
        //         channels: 0,
        //         sdp_fmtp_line: "apt=127".to_owned(),
        //         rtcp_feedback: vec![],
        //     },
        //     payload_type: 120,
        //     ..Default::default()
        // },
        // RTCRtpCodecParameters {
        //     capability: RTCRtpCodecCapability {
        //         mime_type: MIME_TYPE_H264.to_owned(),
        //         clock_rate: 90000,
        //         channels: 0,
        //         sdp_fmtp_line:
        //             "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f"
        //                 .to_owned(),
        //         rtcp_feedback: video_rtcp_feedback.clone(),
        //     },
        //     payload_type: 125,
        //     ..Default::default()
        // },
        // RTCRtpCodecParameters {
        //     capability: RTCRtpCodecCapability {
        //         mime_type: "video/rtx".to_owned(),
        //         clock_rate: 90000,
        //         channels: 0,
        //         sdp_fmtp_line: "apt=125".to_owned(),
        //         rtcp_feedback: vec![],
        //     },
        //     payload_type: 107,
        //     ..Default::default()
        // },
        // RTCRtpCodecParameters {
        //     capability: RTCRtpCodecCapability {
        //         mime_type: MIME_TYPE_H264.to_owned(),
        //         clock_rate: 90000,
        //         channels: 0,
        //         sdp_fmtp_line:
        //             "level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=42e01f"
        //                 .to_owned(),
        //         rtcp_feedback: video_rtcp_feedback.clone(),
        //     },
        //     payload_type: 108,
        //     ..Default::default()
        // },
        // RTCRtpCodecParameters {
        //     capability: RTCRtpCodecCapability {
        //         mime_type: "video/rtx".to_owned(),
        //         clock_rate: 90000,
        //         channels: 0,
        //         sdp_fmtp_line: "apt=108".to_owned(),
        //         rtcp_feedback: vec![],
        //     },
        //     payload_type: 109,
        //     ..Default::default()
        // },
        // RTCRtpCodecParameters {
        //     capability: RTCRtpCodecCapability {
        //         mime_type: MIME_TYPE_H264.to_owned(),
        //         clock_rate: 90000,
        //         channels: 0,
        //         sdp_fmtp_line:
        //             "level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=42001f"
        //                 .to_owned(),
        //         rtcp_feedback: video_rtcp_feedback.clone(),
        //     },
        //     payload_type: 127,
        //     ..Default::default()
        // },
        // RTCRtpCodecParameters {
        //     capability: RTCRtpCodecCapability {
        //         mime_type: "video/rtx".to_owned(),
        //         clock_rate: 90000,
        //         channels: 0,
        //         sdp_fmtp_line: "apt=127".to_owned(),
        //         rtcp_feedback: vec![],
        //     },
        //     payload_type: 120,
        //     ..Default::default()
        // },
        // RTCRtpCodecParameters {
        //     capability: RTCRtpCodecCapability {
        //         mime_type: MIME_TYPE_H264.to_owned(),
        //         clock_rate: 90000,
        //         channels: 0,
        //         sdp_fmtp_line:
        //             "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=640032"
        //                 .to_owned(),
        //         rtcp_feedback: video_rtcp_feedback,
        //     },
        //     payload_type: 123,
        //     ..Default::default()
        // },
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: "video/rtx".to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "apt=123".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 118,
            ..Default::default()
        },
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: "video/ulpfec".to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 116,
            ..Default::default()
        },
    ] {
        media_engine.register_codec(codec, RTPCodecType::Video)?;
    }

    Ok(())
}

pub fn register_rtp_extension_simulcast(m: &mut MediaEngine) -> Result<()> {
    for extension in vec![EXT_URI_SDES_MID, EXT_URI_SDES_RTP_SID, EXT_URI_SDES_REP_SID] {
        m.register_header_extension(
            RTCRtpHeaderExtensionCapability {
                uri: extension.to_owned(),
            },
            RTPCodecType::Video,
            None,
        )?;
    }

    Ok(())
}

pub fn register_rtp_extension_audiolevel(m: &mut MediaEngine) -> Result<()> {
    for extension in vec![
        EXT_URI_SDES_MID,
        EXT_URI_SDES_RTP_SID,
        EXT_URI_SDES_REP_SID,
        EXT_URI_AUDIO_LEVEL,
    ] {
        m.register_header_extension(
            RTCRtpHeaderExtensionCapability {
                uri: extension.to_owned(),
            },
            RTPCodecType::Audio,
            None,
        )?;
    }
    Ok(())
}