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
    aom::{self, AomDecoder, AomDecoderConfig, AomEncoder, AomEncoderConfig},
    common::GoogleImage,
    vpxcodec::{self, VpxDecoder, VpxDecoderConfig, VpxEncoder, VpxEncoderConfig, VpxVideoCodecId},
    CaptureOutputFormat, CodecName, Frame, ImageRgb,
};
#[cfg(feature = "texcodec")]
use crate::{texcodec::*, AdapterDevice};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use hbb_common::sysinfo::{System, SystemExt};
use hbb_common::{
    anyhow::anyhow,
    config::PeerConfig,
    log,
    message_proto::{
        supported_decoding::PreferCodec, video_frame, EncodedVideoFrames, Message,
        SupportedDecoding, SupportedEncoding,
    },
    ResultType,
};
#[cfg(any(feature = "hwcodec", feature = "mediacodec", feature = "texcodec"))]
use hbb_common::{config::Config2, lazy_static};

lazy_static::lazy_static! {
    static ref PEER_DECODINGS: Arc<Mutex<HashMap<i32, SupportedDecoding>>> = Default::default();
    static ref ENCODE_CODEC_NAME: Arc<Mutex<CodecName>> = Arc::new(Mutex::new(CodecName::VP9));
}

pub const INVALID_LUID: i64 = -1;
pub const ENCODE_NEED_SWITCH: &'static str = "ENCODE_NEED_SWITCH";

#[derive(Debug, Clone)]
pub struct HwEncoderConfig {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub bitrate: i32,
}

#[cfg(feature = "texcodec")]
#[derive(Debug, Clone)]
pub struct TexEncoderConfig {
    pub device: AdapterDevice,
    pub width: usize,
    pub height: usize,
    pub bitrate: i32,
    pub feature: texcodec::hw_common::FeatureContext,
}

#[derive(Debug)]
pub enum EncoderCfg {
    VPX(VpxEncoderConfig),
    AOM(AomEncoderConfig),
    #[cfg(feature = "hwcodec")]
    HW(HwEncoderConfig),
    #[cfg(feature = "texcodec")]
    TEX(TexEncoderConfig),
}

pub trait EncoderApi {
    fn new(cfg: EncoderCfg) -> ResultType<Self>
    where
        Self: Sized;

    fn encode_to_message(&mut self, frame: Frame, ms: i64) -> ResultType<Message>;

    fn input_format(&self) -> CaptureOutputFormat;

    fn set_bitrate(&mut self, bitrate: u32) -> ResultType<()>;
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
    vp8: VpxDecoder,
    vp9: VpxDecoder,
    av1: AomDecoder,
    #[cfg(feature = "hwcodec")]
    hw: HwDecoders,
    #[cfg(feature = "texcodec")]
    tex: TexDecoders,
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
    NoTexture,
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
            #[cfg(feature = "texcodec")]
            EncoderCfg::TEX(_) => match TexEncoder::new(config) {
                Ok(tex) => Ok(Encoder {
                    codec: Box::new(tex),
                }),
                Err(e) => {
                    texcodec_new_check_process();
                    *ENCODE_CODEC_NAME.lock().unwrap() = CodecName::VP9;
                    Err(e)
                }
            },
        }
    }

    pub fn update(update: EncodingUpdate) {
        log::info!("update:{:?}", update);
        let mut decodings = PEER_DECODINGS.lock().unwrap();
        let mut _no_texture = false;
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
            EncodingUpdate::NoTexture => {
                _no_texture = true;
            }
        }

        let vp8_useable = decodings.len() > 0 && decodings.iter().all(|(_, s)| s.ability_vp8 > 0);
        let av1_useable = decodings.len() > 0 && decodings.iter().all(|(_, s)| s.ability_av1 > 0);
        #[allow(unused_mut)]
        let mut h264_name = None;
        #[allow(unused_mut)]
        let mut h265_name = None;
        let _h264_useable =
            decodings.len() > 0 && decodings.iter().all(|(_, s)| s.ability_h264 > 0);
        let _h265_useable =
            decodings.len() > 0 && decodings.iter().all(|(_, s)| s.ability_h265 > 0);
        #[cfg(feature = "texcodec")]
        if enable_texcodec_option() && !_no_texture {
            if _h264_useable && h264_name.is_none() {
                if TexEncoder::possible_available(CodecName::H264("".to_string())).len() > 0 {
                    h264_name = Some("".to_string());
                }
            }
            if _h265_useable && h265_name.is_none() {
                if TexEncoder::possible_available(CodecName::H265("".to_string())).len() > 0 {
                    h265_name = Some("".to_string());
                }
            }
        }
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
                    || s.prefer == PreferCodec::H264.into() && h264_name.is_some()
                    || s.prefer == PreferCodec::H265.into() && h265_name.is_some()
            })
            .map(|(_, s)| s.prefer)
            .collect();
        if preferences.len() > 0 && preferences.iter().all(|&p| p == preferences[0]) {
            preference = preferences[0].enum_value_or(PreferCodec::Auto);
        }

        #[allow(unused_mut)]
        let mut auto_codec = CodecName::VP9;
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        if vp8_useable && System::new_all().total_memory() <= 4 * 1024 * 1024 * 1024 {
            // 4 Gb
            auto_codec = CodecName::VP8
        }

        match preference {
            PreferCodec::VP8 => *name = CodecName::VP8,
            PreferCodec::VP9 => *name = CodecName::VP9,
            PreferCodec::AV1 => *name = CodecName::AV1,
            PreferCodec::H264 => *name = h264_name.map_or(auto_codec, |c| CodecName::H264(c)),
            PreferCodec::H265 => *name = h265_name.map_or(auto_codec, |c| CodecName::H265(c)),
            PreferCodec::Auto => *name = auto_codec,
        }

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
        #[cfg(feature = "texcodec")]
        if enable_texcodec_option() {
            encoding.h264 |=
                TexEncoder::possible_available(CodecName::H264("".to_string())).len() > 0;
            encoding.h265 |=
                TexEncoder::possible_available(CodecName::H265("".to_string())).len() > 0;
        }
        encoding
    }

    pub fn fallback(name: CodecName) {
        *ENCODE_CODEC_NAME.lock().unwrap() = name;
    }
}

impl Decoder {
    pub fn supported_decodings(id_for_perfer: Option<&str>, _allow_tex: bool) -> SupportedDecoding {
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
        #[cfg(feature = "texcodec")]
        {
            if enable_texcodec_option() && _allow_tex {
                decoding.ability_h264 |=
                    if TexDecoder::possible_available(CodecName::H264("".to_string())).len() > 0 {
                        1
                    } else {
                        0
                    };
                decoding.ability_h265 |=
                    if TexDecoder::possible_available(CodecName::H265("".to_string())).len() > 0 {
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
            num_threads: (num_cpus::get() / 2) as _,
        })
        .unwrap();
        let vp9 = VpxDecoder::new(VpxDecoderConfig {
            codec: VpxVideoCodecId::VP9,
            num_threads: (num_cpus::get() / 2) as _,
        })
        .unwrap();
        let av1 = AomDecoder::new(AomDecoderConfig {
            num_threads: (num_cpus::get() / 2) as _,
        })
        .unwrap();
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
            #[cfg(feature = "texcodec")]
            tex: if enable_texcodec_option() && _luid != INVALID_LUID {
                TexDecoder::new_decoders(_luid)
            } else {
                TexDecoders::default()
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
                Decoder::handle_vpxs_video_frame(&mut self.vp8, vp8s, rgb)
            }
            video_frame::Union::Vp9s(vp9s) => {
                Decoder::handle_vpxs_video_frame(&mut self.vp9, vp9s, rgb)
            }
            video_frame::Union::Av1s(av1s) => {
                Decoder::handle_av1s_video_frame(&mut self.av1, av1s, rgb)
            }
            #[cfg(any(feature = "hwcodec", feature = "texcodec"))]
            video_frame::Union::H264s(h264s) => {
                #[cfg(feature = "texcodec")]
                {
                    if let Some(decoder) = &mut self.tex.h264 {
                        *_pixelbuffer = false;
                        return Decoder::handle_tex_video_frame(decoder, h264s, _texture);
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
            #[cfg(any(feature = "hwcodec", feature = "texcodec"))]
            video_frame::Union::H265s(h265s) => {
                #[cfg(feature = "texcodec")]
                {
                    if let Some(decoder) = &mut self.tex.h265 {
                        *_pixelbuffer = false;
                        return Decoder::handle_tex_video_frame(decoder, h265s, _texture);
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

    #[cfg(feature = "texcodec")]
    fn handle_tex_video_frame(
        decoder: &mut TexDecoder,
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
fn enable_hwcodec_option() -> bool {
    if let Some(v) = Config2::get().options.get("enable-hwcodec") {
        return v != "N";
    }
    return true; // default is true
}
#[cfg(feature = "texcodec")]
fn enable_texcodec_option() -> bool {
    if let Some(v) = Config2::get().options.get("enable-texcodec") {
        return v != "N";
    }
    return true; // default is true
}
