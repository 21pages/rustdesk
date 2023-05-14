use std::{
    collections::HashMap,
    ffi::c_void,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

#[cfg(feature = "hwcodec")]
use crate::hwcodec::*;
#[cfg(feature = "mediacodec")]
use crate::mediacodec::{
    MediaCodecDecoder, MediaCodecDecoders, H264_DECODER_SUPPORT, H265_DECODER_SUPPORT,
};
use crate::{
    aom::{self, AomDecoder, AomEncoder, AomEncoderConfig},
    common::GoogleImage,
    vpxcodec::{self, VpxDecoder, VpxDecoderConfig, VpxEncoder, VpxEncoderConfig, VpxVideoCodecId},
    CaptureOutputFormat, CodecName, Frame, ImageRgb,
};
#[cfg(feature = "gpu_video_codec")]
use crate::{gpu_video_codec::*, AdapterDevice};

use hbb_common::{
    anyhow::anyhow,
    bail,
    config::PeerConfig,
    log,
    message_proto::{
        supported_decoding::PreferCodec, video_frame, EncodedVideoFrames, Message,
        SupportedDecoding, SupportedEncoding,
    },
    sysinfo::{System, SystemExt},
    tokio::time::Instant,
    ResultType,
};
#[cfg(any(
    feature = "hwcodec",
    feature = "mediacodec",
    feature = "gpu_video_codec"
))]
use hbb_common::{config::Config2, lazy_static};

lazy_static::lazy_static! {
    static ref PEER_DECODINGS: Arc<Mutex<HashMap<i32, SupportedDecoding>>> = Default::default();
    static ref ENCODE_CODEC_NAME: Arc<Mutex<CodecName>> = Arc::new(Mutex::new(CodecName::VP9));
    static ref THREAD_LOG_TIME: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
}

pub const ENCODE_NEED_SWITCH: &'static str = "ENCODE_NEED_SWITCH";

#[derive(Debug, Clone)]
pub struct HwEncoderConfig {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub quality: Quality,
    pub keyframe_interval: Option<usize>,
}

#[cfg(feature = "gpu_video_codec")]
#[derive(Debug, Clone)]
pub struct GvcEncoderConfig {
    pub device: AdapterDevice,
    pub width: usize,
    pub height: usize,
    pub quality: Quality,
    pub feature: gpu_video_codec::gvc_common::FeatureContext,
    pub keyframe_interval: Option<usize>,
}

#[derive(Debug)]
pub enum EncoderCfg {
    VPX(VpxEncoderConfig),
    AOM(AomEncoderConfig),
    #[cfg(feature = "hwcodec")]
    HW(HwEncoderConfig),
    #[cfg(feature = "gpu_video_codec")]
    GVC(GvcEncoderConfig),
}

pub trait EncoderApi {
    fn new(cfg: EncoderCfg) -> ResultType<Self>
    where
        Self: Sized;

    fn encode_to_message(&mut self, frame: Frame, ms: i64) -> ResultType<Message>;

    fn input_format(&self) -> CaptureOutputFormat;

    fn set_quality(&mut self, quality: Quality) -> ResultType<()>;

    fn bitrate(&self) -> u32;
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
    vp8: Option<VpxDecoder>,
    vp9: Option<VpxDecoder>,
    av1: Option<AomDecoder>,
    #[cfg(feature = "hwcodec")]
    hw: HwDecoders,
    #[cfg(feature = "gpu_video_codec")]
    tex: GvcDecoders,
    #[cfg(feature = "hwcodec")]
    i420: Vec<u8>,
    #[cfg(feature = "mediacodec")]
    media_codec: MediaCodecDecoders,
}

#[derive(Debug, Clone)]
pub enum EncodingUpdate {
    New(i32, SupportedDecoding),
    Remove(i32),
    NewOnlyVP9(i32),
    Check,
}

impl Encoder {
    pub fn new(config: EncoderCfg) -> ResultType<Encoder> {
        log::info!("new encoder:{:?}", config);
        match config {
            EncoderCfg::VPX(_) => Ok(Encoder {
                codec: Box::new(VpxEncoder::new(config)?),
            }),
            EncoderCfg::AOM(_) => Ok(Encoder {
                codec: Box::new(AomEncoder::new(config)?),
            }),

            #[cfg(feature = "hwcodec")]
            EncoderCfg::HW(_) => match HwEncoder::new(config) {
                Ok(hw) => Ok(Encoder {
                    codec: Box::new(hw),
                }),
                Err(e) => {
                    hwcodec_new_check_process();
                    *ENCODE_CODEC_NAME.lock().unwrap() = CodecName::VP9;
                    Err(e)
                }
            },
            #[cfg(feature = "gpu_video_codec")]
            EncoderCfg::GVC(_) => match GvcEncoder::new(config) {
                Ok(tex) => Ok(Encoder {
                    codec: Box::new(tex),
                }),
                Err(e) => {
                    gpu_video_codec_new_check_process();
                    *ENCODE_CODEC_NAME.lock().unwrap() = CodecName::VP9;
                    Err(e)
                }
            },
        }
    }

    pub fn update(update: EncodingUpdate) {
        log::info!("update:{:?}", update);
        let mut decodings = PEER_DECODINGS.lock().unwrap();
        match update {
            EncodingUpdate::New(id, decoding) => {
                decodings.insert(id, decoding);
            }
            EncodingUpdate::Remove(id) => {
                decodings.remove(&id);
            }
            EncodingUpdate::NewOnlyVP9(id) => {
                decodings.insert(
                    id,
                    SupportedDecoding {
                        ability_vp9: 1,
                        ..Default::default()
                    },
                );
            }
            EncodingUpdate::Check => {}
        }

        let vp8_useable = decodings.len() > 0 && decodings.iter().all(|(_, s)| s.ability_vp8 > 0);
        let av1_useable = decodings.len() > 0 && decodings.iter().all(|(_, s)| s.ability_av1 > 0);
        let _h264_useable =
            decodings.len() > 0 && decodings.iter().all(|(_, s)| s.ability_h264 > 0);
        let _h265_useable =
            decodings.len() > 0 && decodings.iter().all(|(_, s)| s.ability_h265 > 0);
        let mut h264g = false;
        let mut h265g = false;
        #[cfg(feature = "gpu_video_codec")]
        if enable_gpu_video_codec_option() {
            if _h264_useable {
                if GvcEncoder::possible_available(CodecName::H264G).len() > 0 {
                    h264g = true;
                }
            }
            if _h265_useable {
                if GvcEncoder::possible_available(CodecName::H265G).len() > 0 {
                    h265g = true;
                }
            }
        }
        #[allow(unused_mut)]
        let mut h264_name = None;
        #[allow(unused_mut)]
        let mut h265_name = None;
        #[cfg(feature = "hwcodec")]
        if enable_hwcodec_option() {
            let best = HwEncoder::best();
            if _h264_useable && h264_name.is_none() {
                h264_name = best.h264.map_or(None, |c| Some(c.name));
            }
            if _h265_useable && h265_name.is_none() {
                h265_name = best.h265.map_or(None, |c| Some(c.name));
            }
        }
        let mut name = ENCODE_CODEC_NAME.lock().unwrap();
        let mut preference = PreferCodec::Auto;
        let preferences: Vec<_> = decodings
            .iter()
            .filter(|(_, s)| {
                s.prefer == PreferCodec::VP9.into()
                    || s.prefer == PreferCodec::VP8.into() && vp8_useable
                    || s.prefer == PreferCodec::AV1.into() && av1_useable
                    || s.prefer == PreferCodec::H264.into() && (h264_name.is_some() || h264g)
                    || s.prefer == PreferCodec::H265.into() && (h265_name.is_some() || h265g)
            })
            .map(|(_, s)| s.prefer)
            .collect();
        if preferences.len() > 0 && preferences.iter().all(|&p| p == preferences[0]) {
            preference = preferences[0].enum_value_or(PreferCodec::Auto);
        }

        #[allow(unused_mut)]
        let mut auto_codec = CodecName::VP9;
        if av1_useable {
            auto_codec = CodecName::AV1;
        }
        if vp8_useable && System::new_all().total_memory() <= 4 * 1024 * 1024 * 1024 {
            // 4 Gb
            auto_codec = CodecName::VP8
        }

        *name = match preference {
            PreferCodec::VP8 => CodecName::VP8,
            PreferCodec::VP9 => CodecName::VP9,
            PreferCodec::AV1 => CodecName::AV1,
            PreferCodec::H264 => {
                if h264g {
                    CodecName::H264G
                } else if let Some(v) = h264_name {
                    CodecName::H264(v)
                } else {
                    auto_codec
                }
            }
            PreferCodec::H265 => {
                if h265g {
                    CodecName::H265G
                } else if let Some(v) = h265_name {
                    CodecName::H265(v)
                } else {
                    auto_codec
                }
            }
            PreferCodec::Auto => auto_codec,
        };

        log::info!(
            "connection count:{}, used preference:{:?}, encoder:{:?}",
            decodings.len(),
            preference,
            *name
        )
    }

    #[inline]
    pub fn negotiated_codec() -> CodecName {
        ENCODE_CODEC_NAME.lock().unwrap().clone()
    }

    pub fn supported_encoding() -> SupportedEncoding {
        #[allow(unused_mut)]
        let mut encoding = SupportedEncoding {
            vp8: true,
            av1: true,
            ..Default::default()
        };
        #[cfg(feature = "hwcodec")]
        if enable_hwcodec_option() {
            let best = HwEncoder::best();
            encoding.h264 |= best.h264.is_some();
            encoding.h265 |= best.h265.is_some();
        }
        #[cfg(feature = "gpu_video_codec")]
        if enable_gpu_video_codec_option() {
            encoding.h264 |= GvcEncoder::possible_available(CodecName::H264G).len() > 0;
            encoding.h265 |= GvcEncoder::possible_available(CodecName::H265G).len() > 0;
        }
        encoding
    }

    pub fn fallback(name: CodecName) {
        log::info!("fallback:{:?}", name);
        *ENCODE_CODEC_NAME.lock().unwrap() = name;
    }
}

impl Decoder {
    pub fn supported_decodings(
        id_for_perfer: Option<&str>,
        _allow_tex: bool,
        _luid: Option<i64>,
    ) -> SupportedDecoding {
        #[allow(unused_mut)]
        let mut decoding = SupportedDecoding {
            ability_vp8: 1,
            ability_vp9: 1,
            ability_av1: 1,
            prefer: id_for_perfer
                .map_or(PreferCodec::Auto, |id| Self::codec_preference(id))
                .into(),
            ..Default::default()
        };
        #[cfg(feature = "hwcodec")]
        if enable_hwcodec_option() {
            let best = HwDecoder::best();
            decoding.ability_h264 |= if best.h264.is_some() { 1 } else { 0 };
            decoding.ability_h265 |= if best.h265.is_some() { 1 } else { 0 };
        }
        #[cfg(feature = "gpu_video_codec")]
        {
            if enable_gpu_video_codec_option() && _allow_tex {
                decoding.ability_h264 |=
                    if GvcDecoder::possible_available(CodecName::H264G, _luid).len() > 0 {
                        1
                    } else {
                        0
                    };
                decoding.ability_h265 |=
                    if GvcDecoder::possible_available(CodecName::H265G, _luid).len() > 0 {
                        1
                    } else {
                        0
                    };
            }
        }
        #[cfg(feature = "mediacodec")]
        if enable_hwcodec_option() {
            decoding.ability_h264 =
                if H264_DECODER_SUPPORT.load(std::sync::atomic::Ordering::SeqCst) {
                    1
                } else {
                    0
                };
            decoding.ability_h265 =
                if H265_DECODER_SUPPORT.load(std::sync::atomic::Ordering::SeqCst) {
                    1
                } else {
                    0
                };
        }
        decoding
    }

    pub fn new(_luid: i64) -> Decoder {
        let vp8 = VpxDecoder::new(VpxDecoderConfig {
            codec: VpxVideoCodecId::VP8,
        })
        .ok();
        let vp9 = VpxDecoder::new(VpxDecoderConfig {
            codec: VpxVideoCodecId::VP9,
        })
        .ok();
        let av1 = AomDecoder::new().ok();
        Decoder {
            vp8,
            vp9,
            av1,
            #[cfg(feature = "hwcodec")]
            hw: if enable_hwcodec_option() {
                HwDecoder::new_decoders()
            } else {
                HwDecoders::default()
            },
            #[cfg(feature = "gpu_video_codec")]
            tex: if enable_gpu_video_codec_option() && _luid != 0 {
                GvcDecoder::new_decoders(_luid)
            } else {
                GvcDecoders::default()
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

    // rgb [in/out] fmt and stride must be set in ImageRgb
    pub fn handle_video_frame(
        &mut self,
        frame: &video_frame::Union,
        rgb: &mut ImageRgb,
        _texture: &mut *mut c_void,
        _pixelbuffer: &mut bool,
    ) -> ResultType<bool> {
        match frame {
            video_frame::Union::Vp8s(vp8s) => {
                if let Some(vp8) = &mut self.vp8 {
                    Decoder::handle_vpxs_video_frame(vp8, vp8s, rgb)
                } else {
                    bail!("vp8 decoder not available");
                }
            }
            video_frame::Union::Vp9s(vp9s) => {
                if let Some(vp9) = &mut self.vp9 {
                    Decoder::handle_vpxs_video_frame(vp9, vp9s, rgb)
                } else {
                    bail!("vp9 decoder not available");
                }
            }
            video_frame::Union::Av1s(av1s) => {
                if let Some(av1) = &mut self.av1 {
                    Decoder::handle_av1s_video_frame(av1, av1s, rgb)
                } else {
                    bail!("av1 decoder not available");
                }
            }
            #[cfg(any(feature = "hwcodec", feature = "gpu_video_codec"))]
            video_frame::Union::H264s(h264s) => {
                #[cfg(feature = "gpu_video_codec")]
                {
                    if let Some(decoder) = &mut self.tex.h264 {
                        *_pixelbuffer = false;
                        return Decoder::handle_gvc_video_frame(decoder, h264s, _texture);
                    }
                }
                #[cfg(feature = "hwcodec")]
                {
                    if let Some(decoder) = &mut self.hw.h264 {
                        return Decoder::handle_hw_video_frame(decoder, h264s, rgb, &mut self.i420);
                    }
                }
                Err(anyhow!("don't support h264!"))
            }
            #[cfg(any(feature = "hwcodec", feature = "gpu_video_codec"))]
            video_frame::Union::H265s(h265s) => {
                #[cfg(feature = "gpu_video_codec")]
                {
                    if let Some(decoder) = &mut self.tex.h265 {
                        *_pixelbuffer = false;
                        return Decoder::handle_gvc_video_frame(decoder, h265s, _texture);
                    }
                }
                #[cfg(feature = "hwcodec")]
                {
                    if let Some(decoder) = &mut self.hw.h265 {
                        return Decoder::handle_hw_video_frame(decoder, h265s, rgb, &mut self.i420);
                    }
                }
                Err(anyhow!("don't support h265!"))
            }
            #[cfg(feature = "mediacodec")]
            video_frame::Union::H264s(h264s) => {
                if let Some(decoder) = &mut self.media_codec.h264 {
                    Decoder::handle_mediacodec_video_frame(decoder, h264s, rgb)
                } else {
                    Err(anyhow!("don't support h264!"))
                }
            }
            #[cfg(feature = "mediacodec")]
            video_frame::Union::H265s(h265s) => {
                if let Some(decoder) = &mut self.media_codec.h265 {
                    Decoder::handle_mediacodec_video_frame(decoder, h265s, rgb)
                } else {
                    Err(anyhow!("don't support h265!"))
                }
            }
            _ => Err(anyhow!("unsupported video frame type!")),
        }
    }

    // rgb [in/out] fmt and stride must be set in ImageRgb
    fn handle_vpxs_video_frame(
        decoder: &mut VpxDecoder,
        vpxs: &EncodedVideoFrames,
        rgb: &mut ImageRgb,
    ) -> ResultType<bool> {
        let mut last_frame = vpxcodec::Image::new();
        for vpx in vpxs.frames.iter() {
            for frame in decoder.decode(&vpx.data)? {
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
            last_frame.to(rgb);
            Ok(true)
        }
    }

    // rgb [in/out] fmt and stride must be set in ImageRgb
    fn handle_av1s_video_frame(
        decoder: &mut AomDecoder,
        av1s: &EncodedVideoFrames,
        rgb: &mut ImageRgb,
    ) -> ResultType<bool> {
        let mut last_frame = aom::Image::new();
        for av1 in av1s.frames.iter() {
            for frame in decoder.decode(&av1.data)? {
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
            last_frame.to(rgb);
            Ok(true)
        }
    }

    // rgb [in/out] fmt and stride must be set in ImageRgb
    #[cfg(feature = "hwcodec")]
    fn handle_hw_video_frame(
        decoder: &mut HwDecoder,
        frames: &EncodedVideoFrames,
        rgb: &mut ImageRgb,
        i420: &mut Vec<u8>,
    ) -> ResultType<bool> {
        let mut ret = false;
        for h264 in frames.frames.iter() {
            for image in decoder.decode(&h264.data)? {
                // TODO: just process the last frame
                if image.to_fmt(rgb, i420).is_ok() {
                    ret = true;
                }
            }
        }
        return Ok(ret);
    }

    #[cfg(feature = "gpu_video_codec")]
    fn handle_gvc_video_frame(
        decoder: &mut GvcDecoder,
        frames: &EncodedVideoFrames,
        texture: &mut *mut c_void,
    ) -> ResultType<bool> {
        let mut ret = false;
        for h26x in frames.frames.iter() {
            for image in decoder.decode(&h26x.data)? {
                *texture = image.frame.texture;
                ret = true;
            }
        }
        return Ok(ret);
    }

    // rgb [in/out] fmt and stride must be set in ImageRgb
    #[cfg(feature = "mediacodec")]
    fn handle_mediacodec_video_frame(
        decoder: &mut MediaCodecDecoder,
        frames: &EncodedVideoFrames,
        rgb: &mut ImageRgb,
    ) -> ResultType<bool> {
        let mut ret = false;
        for h264 in frames.frames.iter() {
            return decoder.decode(&h264.data, rgb);
        }
        return Ok(false);
    }

    fn codec_preference(id: &str) -> PreferCodec {
        let codec = PeerConfig::load(id)
            .options
            .get("codec-preference")
            .map_or("".to_owned(), |c| c.to_owned());
        if codec == "vp8" {
            PreferCodec::VP8
        } else if codec == "vp9" {
            PreferCodec::VP9
        } else if codec == "av1" {
            PreferCodec::AV1
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
pub fn enable_hwcodec_option() -> bool {
    if let Some(v) = Config2::get().options.get("enable-hwcodec") {
        return v != "N";
    }
    return true; // default is true
}
#[cfg(feature = "gpu_video_codec")]
fn enable_gpu_video_codec_option() -> bool {
    if let Some(v) = Config2::get().options.get("enable-gpu-video-codec") {
        return v != "N";
    }
    return true; // default is true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Best,
    Balanced,
    Low,
    Custom(u32),
}

impl Default for Quality {
    fn default() -> Self {
        Self::Balanced
    }
}

pub fn base_bitrate(width: u32, height: u32) -> u32 {
    #[allow(unused_mut)]
    let mut base_bitrate = ((width * height) / 1000) as u32; // same as 1.1.9
    if base_bitrate == 0 {
        base_bitrate = 1920 * 1080 / 1000;
    }
    #[cfg(target_os = "android")]
    {
        // fix when android screen shrinks
        let fix = crate::Display::fix_quality() as u32;
        log::debug!("Android screen, fix quality:{}", fix);
        base_bitrate = base_bitrate * fix;
    }
    base_bitrate
}

pub fn codec_thread_num() -> usize {
    let max: usize = num_cpus::get();
    let mut res;
    let info;
    #[cfg(windows)]
    {
        res = 0;
        let percent = hbb_common::platform::windows::cpu_uage_one_minute();
        info = format!("cpu usage:{:?}", percent);
        if let Some(pecent) = percent {
            if pecent < 100.0 {
                res = ((100.0 - pecent) * (max as f64) / 200.0).round() as usize;
            }
        }
    }
    #[cfg(not(windows))]
    {
        let s = System::new_all();
        // https://man7.org/linux/man-pages/man3/getloadavg.3.html
        let avg = s.load_average();
        info = format!("cpu loadavg:{}", avg.one);
        res = (((max as f64) - avg.one) * 0.5).round() as usize;
    }
    res = std::cmp::min(res, max / 2);
    if res == 0 {
        res = 1;
    }
    // avoid frequent log
    let log = match THREAD_LOG_TIME.lock().unwrap().clone() {
        Some(instant) => instant.elapsed().as_secs() > 1,
        None => true,
    };
    if log {
        log::info!("cpu num:{max}, {info}, codec thread:{res}");
        *THREAD_LOG_TIME.lock().unwrap() = Some(Instant::now());
    }
    res
}
