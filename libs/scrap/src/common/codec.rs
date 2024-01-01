use std::{
    collections::HashMap,
    ffi::c_void,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

#[cfg(feature = "gpucodec")]
use crate::gpucodec::*;
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
    CodecName, EncodeInput, EncodeYuvFormat, ImageRgb,
};

use hbb_common::{
    anyhow::anyhow,
    bail,
    config::PeerConfig,
    log,
    message_proto::{
        supported_decoding::PreferCodec, video_frame, Chroma, CodecAbility, EncodedVideoFrames,
        SupportedDecoding, SupportedEncoding, VideoFrame,
    },
    sysinfo::System,
    tokio::time::Instant,
    ResultType,
};
#[cfg(any(feature = "hwcodec", feature = "mediacodec", feature = "gpucodec"))]
use hbb_common::{config::Config2, lazy_static};

lazy_static::lazy_static! {
    static ref PEER_DECODINGS: Arc<Mutex<HashMap<i32, SupportedDecoding>>> = Default::default();
    static ref ENCODE_CODEC_NAME: Arc<Mutex<CodecName>> = Arc::new(Mutex::new(CodecName::VP9));
    static ref THREAD_LOG_TIME: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
}

pub const ENCODE_NEED_SWITCH: &'static str = "ENCODE_NEED_SWITCH";

#[derive(Debug, Clone)]
pub enum EncoderCfg {
    VPX(VpxEncoderConfig),
    AOM(AomEncoderConfig),
    #[cfg(feature = "hwcodec")]
    HW(HwEncoderConfig),
    #[cfg(feature = "gpucodec")]
    GPU(GpuEncoderConfig),
}

pub trait EncoderApi {
    fn new(cfg: EncoderCfg, i444: bool) -> ResultType<Self>
    where
        Self: Sized;

    fn encode_to_message(&mut self, frame: EncodeInput, ms: i64) -> ResultType<VideoFrame>;

    fn yuvfmt(&self) -> EncodeYuvFormat;

    #[cfg(feature = "gpucodec")]
    fn input_texture(&self) -> bool;

    fn set_quality(&mut self, quality: Quality) -> ResultType<()>;

    fn bitrate(&self) -> u32;

    fn support_abr(&self) -> bool;
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
    #[cfg(feature = "gpucodec")]
    gpu: GpuDecoders,
    #[cfg(feature = "hwcodec")]
    i420: Vec<u8>,
    #[cfg(feature = "mediacodec")]
    media_codec: MediaCodecDecoders,
}

#[derive(Debug, Clone)]
pub enum EncodingUpdate {
    Update(i32, SupportedDecoding),
    Remove(i32),
    NewOnlyVP9(i32),
    Check,
}

impl Encoder {
    pub fn new(config: EncoderCfg, i444: bool) -> ResultType<Encoder> {
        log::info!("new encoder: {config:?}, i444: {i444}");
        match config {
            EncoderCfg::VPX(_) => Ok(Encoder {
                codec: Box::new(VpxEncoder::new(config, i444)?),
            }),
            EncoderCfg::AOM(_) => Ok(Encoder {
                codec: Box::new(AomEncoder::new(config, i444)?),
            }),

            #[cfg(feature = "hwcodec")]
            EncoderCfg::HW(_) => match HwEncoder::new(config, i444) {
                Ok(hw) => Ok(Encoder {
                    codec: Box::new(hw),
                }),
                Err(e) => {
                    log::error!("new hw encoder failed: {e:?}, clear config");
                    hbb_common::config::HwCodecConfig::clear();
                    *ENCODE_CODEC_NAME.lock().unwrap() = CodecName::VP9;
                    Err(e)
                }
            },
            #[cfg(feature = "gpucodec")]
            EncoderCfg::GPU(_) => match GpuEncoder::new(config, i444) {
                Ok(tex) => Ok(Encoder {
                    codec: Box::new(tex),
                }),
                Err(e) => {
                    log::error!("new gpu encoder failed: {e:?}, clear config");
                    hbb_common::config::GpucodecConfig::clear();
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
            EncodingUpdate::Update(id, decoding) => {
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
        let _all_support_h264_decoding =
            decodings.len() > 0 && decodings.iter().all(|(_, s)| s.ability_h264 > 0);
        let _all_support_h265_decoding =
            decodings.len() > 0 && decodings.iter().all(|(_, s)| s.ability_h265 > 0);
        #[allow(unused_mut)]
        let mut h264gpu_encoding = false;
        #[allow(unused_mut)]
        let mut h265gpu_encoding = false;
        #[cfg(feature = "gpucodec")]
        if enable_gpucodec_option() {
            if _all_support_h264_decoding {
                if GpuEncoder::available(CodecName::H264GPU).len() > 0 {
                    h264gpu_encoding = true;
                }
            }
            if _all_support_h265_decoding {
                if GpuEncoder::available(CodecName::H265GPU).len() > 0 {
                    h265gpu_encoding = true;
                }
            }
        }
        #[allow(unused_mut)]
        let mut h264hw_encoding = None;
        #[allow(unused_mut)]
        let mut h265hw_encoding = None;
        #[cfg(feature = "hwcodec")]
        if enable_hwcodec_option() {
            let best = HwEncoder::best();
            if _all_support_h264_decoding {
                h264hw_encoding = best.h264.map_or(None, |c| Some(c.name));
            }
            if _all_support_h265_decoding {
                h265hw_encoding = best.h265.map_or(None, |c| Some(c.name));
            }
        }
        let h264_useable =
            _all_support_h264_decoding && (h264gpu_encoding || h264hw_encoding.is_some());
        let h265_useable =
            _all_support_h265_decoding && (h265gpu_encoding || h265hw_encoding.is_some());
        let mut name = ENCODE_CODEC_NAME.lock().unwrap();
        let mut preference = PreferCodec::Auto;
        let preferences: Vec<_> = decodings
            .iter()
            .filter(|(_, s)| {
                s.prefer == PreferCodec::VP9.into()
                    || s.prefer == PreferCodec::VP8.into() && vp8_useable
                    || s.prefer == PreferCodec::AV1.into() && av1_useable
                    || s.prefer == PreferCodec::H264.into() && h264_useable
                    || s.prefer == PreferCodec::H265.into() && h265_useable
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
        let mut system = System::new();
        system.refresh_memory();
        if vp8_useable && system.total_memory() <= 4 * 1024 * 1024 * 1024 {
            // 4 Gb
            auto_codec = CodecName::VP8
        }

        *name = match preference {
            PreferCodec::VP8 => CodecName::VP8,
            PreferCodec::VP9 => CodecName::VP9,
            PreferCodec::AV1 => CodecName::AV1,
            PreferCodec::H264 => {
                if h264gpu_encoding {
                    CodecName::H264GPU
                } else if let Some(v) = h264hw_encoding {
                    CodecName::H264HW(v)
                } else {
                    auto_codec
                }
            }
            PreferCodec::H265 => {
                if h265gpu_encoding {
                    CodecName::H265GPU
                } else if let Some(v) = h265hw_encoding {
                    CodecName::H265HW(v)
                } else {
                    auto_codec
                }
            }
            PreferCodec::Auto => auto_codec,
        };
        if decodings.len() > 0 {
            log::info!(
                "usable: vp8={vp8_useable}, av1={av1_useable}, h264={h264_useable}, h265={h265_useable}",
            );
            log::info!(
                "connection count: {}, used preference: {:?}, encoder: {:?}",
                decodings.len(),
                preference,
                *name
            )
        }
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
            i444: Some(CodecAbility {
                vp9: true,
                av1: true,
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };
        #[cfg(feature = "hwcodec")]
        if enable_hwcodec_option() {
            let best = HwEncoder::best();
            encoding.h264 |= best.h264.is_some();
            encoding.h265 |= best.h265.is_some();
        }
        #[cfg(feature = "gpucodec")]
        if enable_gpucodec_option() {
            encoding.h264 |= GpuEncoder::available(CodecName::H264GPU).len() > 0;
            encoding.h265 |= GpuEncoder::available(CodecName::H265GPU).len() > 0;
        }
        encoding
    }

    pub fn set_fallback(config: &EncoderCfg) {
        let name = match config {
            EncoderCfg::VPX(vpx) => match vpx.codec {
                VpxVideoCodecId::VP8 => CodecName::VP8,
                VpxVideoCodecId::VP9 => CodecName::VP9,
            },
            EncoderCfg::AOM(_) => CodecName::AV1,
            #[cfg(feature = "hwcodec")]
            EncoderCfg::HW(hw) => {
                if hw.name.to_lowercase().contains("h264") {
                    CodecName::H264HW(hw.name.clone())
                } else {
                    CodecName::H265HW(hw.name.clone())
                }
            }
            #[cfg(feature = "gpucodec")]
            EncoderCfg::GPU(gpu) => match gpu.feature.data_format {
                gpucodec::gpu_common::DataFormat::H264 => CodecName::H264GPU,
                gpucodec::gpu_common::DataFormat::H265 => CodecName::H265GPU,
                _ => {
                    log::error!(
                        "should not reach here, gpucodec not support {:?}",
                        gpu.feature.data_format
                    );
                    return;
                }
            },
        };
        let current = ENCODE_CODEC_NAME.lock().unwrap().clone();
        if current != name {
            log::info!("codec fallback: {:?} -> {:?}", current, name);
            *ENCODE_CODEC_NAME.lock().unwrap() = name;
        }
    }

    pub fn use_i444(config: &EncoderCfg) -> bool {
        let decodings = PEER_DECODINGS.lock().unwrap().clone();
        let prefer_i444 = decodings
            .iter()
            .all(|d| d.1.prefer_chroma == Chroma::I444.into());
        let i444_useable = match config {
            EncoderCfg::VPX(vpx) => match vpx.codec {
                VpxVideoCodecId::VP8 => false,
                VpxVideoCodecId::VP9 => decodings.iter().all(|d| d.1.i444.vp9),
            },
            EncoderCfg::AOM(_) => decodings.iter().all(|d| d.1.i444.av1),
            #[cfg(feature = "hwcodec")]
            EncoderCfg::HW(_) => false,
            #[cfg(feature = "gpucodec")]
            EncoderCfg::GPU(_) => false,
        };
        prefer_i444 && i444_useable && !decodings.is_empty()
    }
}

impl Decoder {
    pub fn supported_decodings(
        id_for_perfer: Option<&str>,
        _flutter: bool,
        _luid: Option<i64>,
    ) -> SupportedDecoding {
        let (prefer, prefer_chroma) = Self::preference(id_for_perfer);

        #[allow(unused_mut)]
        let mut decoding = SupportedDecoding {
            ability_vp8: 1,
            ability_vp9: 1,
            ability_av1: 1,
            i444: Some(CodecAbility {
                vp9: true,
                av1: true,
                ..Default::default()
            })
            .into(),
            prefer: prefer.into(),
            prefer_chroma: prefer_chroma.into(),
            ..Default::default()
        };
        #[cfg(feature = "hwcodec")]
        if enable_hwcodec_option() {
            let best = HwDecoder::best();
            decoding.ability_h264 |= if best.h264.is_some() { 1 } else { 0 };
            decoding.ability_h265 |= if best.h265.is_some() { 1 } else { 0 };
        }
        #[cfg(feature = "gpucodec")]
        if enable_gpucodec_option() && _flutter {
            decoding.ability_h264 |= if GpuDecoder::available(CodecName::H264GPU, _luid).len() > 0 {
                1
            } else {
                0
            };
            decoding.ability_h265 |= if GpuDecoder::available(CodecName::H265GPU, _luid).len() > 0 {
                1
            } else {
                0
            };
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

    pub fn exist_codecs(&self, _flutter: bool) -> CodecAbility {
        #[allow(unused_mut)]
        let mut ability = CodecAbility {
            vp8: self.vp8.is_some(),
            vp9: self.vp9.is_some(),
            av1: self.av1.is_some(),
            ..Default::default()
        };
        #[cfg(feature = "hwcodec")]
        {
            ability.h264 |= self.hw.h264.is_some();
            ability.h265 |= self.hw.h265.is_some();
        }
        #[cfg(feature = "gpucodec")]
        if _flutter {
            ability.h264 |= self.gpu.h264.is_some();
            ability.h265 |= self.gpu.h265.is_some();
        }
        #[cfg(feature = "mediacodec")]
        {
            ability.h264 = self.media_codec.h264.is_some();
            ability.h265 = self.media_codec.h265.is_some();
        }
        ability
    }

    pub fn new(_luid: Option<i64>) -> Decoder {
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
            #[cfg(feature = "gpucodec")]
            gpu: if enable_gpucodec_option() && _luid.clone().unwrap_or_default() != 0 {
                GpuDecoder::new_decoders(_luid)
            } else {
                GpuDecoders::default()
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
        chroma: &mut Option<Chroma>,
    ) -> ResultType<bool> {
        match frame {
            video_frame::Union::Vp8s(vp8s) => {
                if let Some(vp8) = &mut self.vp8 {
                    Decoder::handle_vpxs_video_frame(vp8, vp8s, rgb, chroma)
                } else {
                    bail!("vp8 decoder not available");
                }
            }
            video_frame::Union::Vp9s(vp9s) => {
                if let Some(vp9) = &mut self.vp9 {
                    Decoder::handle_vpxs_video_frame(vp9, vp9s, rgb, chroma)
                } else {
                    bail!("vp9 decoder not available");
                }
            }
            video_frame::Union::Av1s(av1s) => {
                if let Some(av1) = &mut self.av1 {
                    Decoder::handle_av1s_video_frame(av1, av1s, rgb, chroma)
                } else {
                    bail!("av1 decoder not available");
                }
            }
            #[cfg(any(feature = "hwcodec", feature = "gpucodec"))]
            video_frame::Union::H264s(h264s) => {
                *chroma = Some(Chroma::I420);
                #[cfg(feature = "gpucodec")]
                if let Some(decoder) = &mut self.gpu.h264 {
                    *_pixelbuffer = false;
                    return Decoder::handle_gpu_video_frame(decoder, h264s, _texture);
                }
                #[cfg(feature = "hwcodec")]
                if let Some(decoder) = &mut self.hw.h264 {
                    return Decoder::handle_hw_video_frame(decoder, h264s, rgb, &mut self.i420);
                }
                Err(anyhow!("don't support h264!"))
            }
            #[cfg(any(feature = "hwcodec", feature = "gpucodec"))]
            video_frame::Union::H265s(h265s) => {
                *chroma = Some(Chroma::I420);
                #[cfg(feature = "gpucodec")]
                if let Some(decoder) = &mut self.gpu.h265 {
                    *_pixelbuffer = false;
                    return Decoder::handle_gpu_video_frame(decoder, h265s, _texture);
                }
                #[cfg(feature = "hwcodec")]
                if let Some(decoder) = &mut self.hw.h265 {
                    return Decoder::handle_hw_video_frame(decoder, h265s, rgb, &mut self.i420);
                }
                Err(anyhow!("don't support h265!"))
            }
            #[cfg(feature = "mediacodec")]
            video_frame::Union::H264s(h264s) => {
                *chroma = Some(Chroma::I420);
                if let Some(decoder) = &mut self.media_codec.h264 {
                    Decoder::handle_mediacodec_video_frame(decoder, h264s, rgb)
                } else {
                    Err(anyhow!("don't support h264!"))
                }
            }
            #[cfg(feature = "mediacodec")]
            video_frame::Union::H265s(h265s) => {
                *chroma = Some(Chroma::I420);
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
        chroma: &mut Option<Chroma>,
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
            *chroma = Some(last_frame.chroma());
            last_frame.to(rgb);
            Ok(true)
        }
    }

    // rgb [in/out] fmt and stride must be set in ImageRgb
    fn handle_av1s_video_frame(
        decoder: &mut AomDecoder,
        av1s: &EncodedVideoFrames,
        rgb: &mut ImageRgb,
        chroma: &mut Option<Chroma>,
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
            *chroma = Some(last_frame.chroma());
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

    #[cfg(feature = "gpucodec")]
    fn handle_gpu_video_frame(
        decoder: &mut GpuDecoder,
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

    fn preference(id: Option<&str>) -> (PreferCodec, Chroma) {
        let id = id.unwrap_or_default();
        if id.is_empty() {
            return (PreferCodec::Auto, Chroma::I420);
        }
        let options = PeerConfig::load(id).options;
        let codec = options
            .get("codec-preference")
            .map_or("".to_owned(), |c| c.to_owned());
        let codec = if codec == "vp8" {
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
        };
        let chroma = if options.get("i444") == Some(&"Y".to_string()) {
            Chroma::I444
        } else {
            Chroma::I420
        };
        (codec, chroma)
    }
}

#[cfg(any(feature = "hwcodec", feature = "mediacodec"))]
pub fn enable_hwcodec_option() -> bool {
    if let Some(v) = Config2::get().options.get("enable-hwcodec") {
        return v != "N";
    }
    return true; // default is true
}
#[cfg(feature = "gpucodec")]
pub fn enable_gpucodec_option() -> bool {
    if let Some(v) = Config2::get().options.get("enable-gpucodec") {
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

pub fn codec_thread_num(limit: usize) -> usize {
    let max: usize = num_cpus::get();
    let mut res;
    let info;
    let mut s = System::new();
    s.refresh_memory();
    let memory = s.available_memory() / 1024 / 1024 / 1024;
    #[cfg(windows)]
    {
        res = 0;
        let percent = hbb_common::platform::windows::cpu_uage_one_minute();
        info = format!("cpu usage: {:?}", percent);
        if let Some(pecent) = percent {
            if pecent < 100.0 {
                res = ((100.0 - pecent) * (max as f64) / 200.0).round() as usize;
            }
        }
    }
    #[cfg(not(windows))]
    {
        s.refresh_cpu_usage();
        // https://man7.org/linux/man-pages/man3/getloadavg.3.html
        let avg = s.load_average();
        info = format!("cpu loadavg: {}", avg.one);
        res = (((max as f64) - avg.one) * 0.5).round() as usize;
    }
    res = std::cmp::min(res, max / 2);
    res = std::cmp::min(res, memory as usize / 2);
    //  Use common thread count
    res = match res {
        _ if res >= 64 => 64,
        _ if res >= 32 => 32,
        _ if res >= 16 => 16,
        _ if res >= 8 => 8,
        _ if res >= 4 => 4,
        _ if res >= 2 => 2,
        _ => 1,
    };
    // https://aomedia.googlesource.com/aom/+/refs/heads/main/av1/av1_cx_iface.c#677
    // https://aomedia.googlesource.com/aom/+/refs/heads/main/aom_util/aom_thread.h#26
    // https://chromium.googlesource.com/webm/libvpx/+/refs/heads/main/vp8/vp8_cx_iface.c#148
    // https://chromium.googlesource.com/webm/libvpx/+/refs/heads/main/vp9/vp9_cx_iface.c#190
    // https://github.com/FFmpeg/FFmpeg/blob/7c16bf0829802534004326c8e65fb6cdbdb634fa/libavcodec/pthread.c#L65
    // https://github.com/FFmpeg/FFmpeg/blob/7c16bf0829802534004326c8e65fb6cdbdb634fa/libavcodec/pthread_internal.h#L26
    // libaom: MAX_NUM_THREADS = 64
    // libvpx: MAX_NUM_THREADS = 64
    // ffmpeg: MAX_AUTO_THREADS = 16
    res = std::cmp::min(res, limit);
    // avoid frequent log
    let log = match THREAD_LOG_TIME.lock().unwrap().clone() {
        Some(instant) => instant.elapsed().as_secs() > 1,
        None => true,
    };
    if log {
        log::info!("cpu num: {max}, {info}, available memory: {memory}G, codec thread: {res}");
        *THREAD_LOG_TIME.lock().unwrap() = Some(Instant::now());
    }
    res
}
