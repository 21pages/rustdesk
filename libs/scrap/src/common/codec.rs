use std::ops::{Deref, DerefMut};
#[cfg(feature = "hwcodec")]
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[cfg(feature = "hwcodec")]
use crate::hwcodec::*;
#[cfg(feature = "mediacodec")]
use crate::mediacodec::{
    MediaCodecDecoder, MediaCodecDecoders, H264_DECODER_SUPPORT, H265_DECODER_SUPPORT,
};
use crate::{vpxcodec::*, ImageFormat};
use codec_common::EncodeContext;

use hbb_common::{
    anyhow::anyhow,
    log,
    message_proto::{video_frame, EncodedVideoFrames, Message, VideoCodecState},
    ResultType,
};
#[cfg(any(feature = "hwcodec", feature = "mediacodec"))]
use hbb_common::{
    config::{Config2, PeerConfig},
    lazy_static,
    message_proto::video_codec_state::PreferCodec,
};

#[cfg(feature = "hwcodec")]
lazy_static::lazy_static! {
    static ref PEER_DECODER_STATES: Arc<Mutex<HashMap<i32, VideoCodecState>>> = Default::default();
}

#[derive(Debug, Clone)]
pub enum EncoderCfg {
    VPX(VpxEncoderConfig),
    HW(EncodeContext),
}

pub trait EncoderApi {
    fn new(cfg: EncoderCfg) -> ResultType<Self>
    where
        Self: Sized;

    fn encode_to_message(&mut self, frame: &[u8], ms: i64) -> ResultType<Message>;

    fn use_yuv(&self) -> bool;

    fn set_bitrate(&mut self, bitrate: u32) -> ResultType<()>;
}

pub struct DecoderCfg {
    pub vpx: VpxDecoderConfig,
}

pub struct Encoder {
    pub codec: Box<dyn EncoderApi>,
}

impl Deref for Encoder {
    type Target = Box<dyn EncoderApi>;

    fn deref(&self) -> &Self::Target {
        &self.codec
    }
}

impl DerefMut for Encoder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.codec
    }
}

pub struct Decoder {
    vpx: VpxDecoder,
    #[cfg(feature = "hwcodec")]
    hw: HwDecoders,
    #[cfg(feature = "hwcodec")]
    i420: Vec<u8>,
    #[cfg(feature = "mediacodec")]
    media_codec: MediaCodecDecoders,
}

#[derive(Debug, Clone)]
pub enum EncoderUpdate {
    State(VideoCodecState),
    Remove,
    DisableHwIfNotExist,
}

impl Encoder {
    pub fn new(config: EncoderCfg) -> ResultType<Encoder> {
        log::info!("new encoder:{:?}", config);
        match config {
            EncoderCfg::VPX(_) => Ok(Encoder {
                codec: Box::new(VpxEncoder::new(config)?),
            }),

            #[cfg(feature = "hwcodec")]
            EncoderCfg::HW(_) => match HwEncoder::new(config) {
                Ok(hw) => Ok(Encoder {
                    codec: Box::new(hw),
                }),
                Err(e) => {
                    check_config_process(true);
                    Err(e)
                }
            },
            #[cfg(not(feature = "hwcodec"))]
            _ => Err(anyhow!("unsupported encoder type")),
        }
    }

    // TODO
    pub fn update_video_encoder(id: i32, update: EncoderUpdate) {
        #[cfg(feature = "hwcodec")]
        {
            let mut states = PEER_DECODER_STATES.lock().unwrap();
            match update {
                EncoderUpdate::State(state) => {
                    states.insert(id, state);
                }
                EncoderUpdate::Remove => {
                    states.remove(&id);
                }
                EncoderUpdate::DisableHwIfNotExist => {
                    if !states.contains_key(&id) {
                        states.insert(id, VideoCodecState::default());
                    }
                }
            }
            let current = HwEncoder::current();
            if states.len() > 0 {
                let best = HwEncoder::best();
                let enabled_h264 = best.h264.is_some()
                    && states.len() > 0
                    && states.iter().all(|(_, s)| s.score_h264 > 0);
                let enabled_h265 = best.h265.is_some()
                    && states.len() > 0
                    && states.iter().all(|(_, s)| s.score_h265 > 0);

                // Preference first
                let mut preference = PreferCodec::Auto;
                let preferences: Vec<_> = states
                    .iter()
                    .filter(|(_, s)| {
                        s.prefer == PreferCodec::VPX.into()
                            || s.prefer == PreferCodec::H264.into() && enabled_h264
                            || s.prefer == PreferCodec::H265.into() && enabled_h265
                    })
                    .map(|(_, s)| s.prefer)
                    .collect();
                if preferences.len() > 0 && preferences.iter().all(|&p| p == preferences[0]) {
                    preference = preferences[0].enum_value_or(PreferCodec::Auto);
                }

                *current.lock().unwrap() = match preference {
                    PreferCodec::VPX => None,
                    PreferCodec::H264 => best.h264,
                    PreferCodec::H265 => best.h265,
                    PreferCodec::Auto => None,
                };

                log::info!(
                    "connection count:{}, used preference:{:?}, encoder:{:?}",
                    states.len(),
                    preference,
                    current.lock().unwrap()
                )
            } else {
                *current.lock().unwrap() = None;
            }
        }
        #[cfg(not(feature = "hwcodec"))]
        {
            let _ = id;
            let _ = update;
        }
    }
    #[inline]
    pub fn current_hw_encoder() -> Option<EncodeContext> {
        #[cfg(feature = "hwcodec")]
        if enable_hwcodec_option() {
            return HwEncoder::current().lock().unwrap().clone();
        } else {
            return None;
        }
        #[cfg(not(feature = "hwcodec"))]
        return None;
    }

    pub fn supported_encoding() -> (bool, bool) {
        #[cfg(feature = "hwcodec")]
        if enable_hwcodec_option() {
            let best = HwEncoder::best();
            (best.h264.is_some(), best.h265.is_some())
        } else {
            (false, false)
        }
        #[cfg(not(feature = "hwcodec"))]
        (false, false)
    }
}

impl Decoder {
    pub fn video_codec_state(_id: &str) -> VideoCodecState {
        #[cfg(feature = "hwcodec")]
        if enable_hwcodec_option() {
            let best = HwDecoder::best();
            // to-do: replace score
            return VideoCodecState {
                score_vpx: 1,
                score_h264: if best.h264.is_some() { 1 } else { 0 },
                score_h265: if best.h265.is_some() { 1 } else { 0 },
                prefer: Self::codec_preference(_id).into(),
                ..Default::default()
            };
        }
        #[cfg(feature = "mediacodec")]
        if enable_hwcodec_option() {
            let score_h264 = if H264_DECODER_SUPPORT.load(std::sync::atomic::Ordering::SeqCst) {
                92
            } else {
                0
            };
            let score_h265 = if H265_DECODER_SUPPORT.load(std::sync::atomic::Ordering::SeqCst) {
                94
            } else {
                0
            };
            return VideoCodecState {
                score_vpx: 1,
                score_h264,
                score_h265,
                prefer: Self::codec_preference(_id).into(),
                ..Default::default()
            };
        }
        VideoCodecState {
            score_vpx: 1,
            ..Default::default()
        }
    }

    pub fn new(config: DecoderCfg) -> Decoder {
        let vpx = VpxDecoder::new(config.vpx).unwrap();
        Decoder {
            vpx,
            #[cfg(feature = "hwcodec")]
            hw: if enable_hwcodec_option() {
                HwDecoder::new_decoders()
            } else {
                HwDecoders::default()
            },
            #[cfg(feature = "hwcodec")]
            i420: vec![],
            #[cfg(feature = "mediacodec")]
            media_codec: if enable_hwcodec_option() {
                MediaCodecDecoder::new_decoders()
            } else {
                MediaCodecDecoders::default()
            },
        }
    }

    pub fn handle_video_frame(
        &mut self,
        frame: &video_frame::Union,
        fmt: (ImageFormat, usize),
        rgb: &mut Vec<u8>,
    ) -> ResultType<bool> {
        match frame {
            video_frame::Union::Vp9s(vp9s) => {
                Decoder::handle_vp9s_video_frame(&mut self.vpx, vp9s, fmt, rgb)
            }
            #[cfg(feature = "hwcodec")]
            video_frame::Union::H264s(h264s) => {
                if let Some(decoder) = &mut self.hw.h264 {
                    Decoder::handle_hw_video_frame(decoder, h264s, fmt, rgb, &mut self.i420)
                } else {
                    Err(anyhow!("don't support h264!"))
                }
            }
            #[cfg(feature = "hwcodec")]
            video_frame::Union::H265s(h265s) => {
                if let Some(decoder) = &mut self.hw.h265 {
                    Decoder::handle_hw_video_frame(decoder, h265s, fmt, rgb, &mut self.i420)
                } else {
                    Err(anyhow!("don't support h265!"))
                }
            }
            #[cfg(feature = "mediacodec")]
            video_frame::Union::H264s(h264s) => {
                if let Some(decoder) = &mut self.media_codec.h264 {
                    Decoder::handle_mediacodec_video_frame(decoder, h264s, fmt, rgb)
                } else {
                    Err(anyhow!("don't support h264!"))
                }
            }
            #[cfg(feature = "mediacodec")]
            video_frame::Union::H265s(h265s) => {
                if let Some(decoder) = &mut self.media_codec.h265 {
                    Decoder::handle_mediacodec_video_frame(decoder, h265s, fmt, rgb)
                } else {
                    Err(anyhow!("don't support h265!"))
                }
            }
            _ => Err(anyhow!("unsupported video frame type!")),
        }
    }

    fn handle_vp9s_video_frame(
        decoder: &mut VpxDecoder,
        vp9s: &EncodedVideoFrames,
        fmt: (ImageFormat, usize),
        rgb: &mut Vec<u8>,
    ) -> ResultType<bool> {
        let mut last_frame = Image::new();
        for vp9 in vp9s.frames.iter() {
            for frame in decoder.decode(&vp9.data)? {
                drop(last_frame);
                last_frame = frame;
            }
        }
        for frame in decoder.flush()? {
            drop(last_frame);
            last_frame = frame;
        }
        if last_frame.is_null() {
            Ok(false)
        } else {
            last_frame.to(fmt.0, fmt.1, rgb);
            Ok(true)
        }
    }

    #[cfg(feature = "hwcodec")]
    fn handle_hw_video_frame(
        decoder: &mut HwDecoder,
        frames: &EncodedVideoFrames,
        fmt: (ImageFormat, usize),
        raw: &mut Vec<u8>,
        i420: &mut Vec<u8>,
    ) -> ResultType<bool> {
        let mut ret = false;
        for h264 in frames.frames.iter() {
            for image in decoder.decode(&h264.data)? {
                // TODO: just process the last frame
                if image.to_fmt(fmt, raw, i420).is_ok() {
                    ret = true;
                }
            }
        }
        return Ok(ret);
    }

    #[cfg(feature = "mediacodec")]
    fn handle_mediacodec_video_frame(
        decoder: &mut MediaCodecDecoder,
        frames: &EncodedVideoFrames,
        fmt: (ImageFormat, usize),
        raw: &mut Vec<u8>,
    ) -> ResultType<bool> {
        let mut ret = false;
        for h264 in frames.frames.iter() {
            return decoder.decode(&h264.data, fmt, raw);
        }
        return Ok(false);
    }

    #[cfg(any(feature = "hwcodec", feature = "mediacodec"))]
    fn codec_preference(id: &str) -> PreferCodec {
        let codec = PeerConfig::load(id)
            .options
            .get("codec-preference")
            .map_or("".to_owned(), |c| c.to_owned());
        if codec == "vp9" {
            PreferCodec::VPX
        } else if codec == "h264" {
            PreferCodec::H264
        } else if codec == "h265" {
            PreferCodec::H265
        } else {
            PreferCodec::Auto
        }
    }
}

#[cfg(any(feature = "hwcodec", feature = "mediacodec"))]
fn enable_hwcodec_option() -> bool {
    if let Some(v) = Config2::get().options.get("enable-hwcodec") {
        return v != "N";
    }
    return true; // default is true
}
