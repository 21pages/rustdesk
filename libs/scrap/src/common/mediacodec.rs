use hbb_common::{anyhow::Error, bail, log, ResultType};
use ndk::media::media_codec::{MediaCodec, MediaCodecDirection, MediaFormat};
use std::ops::Deref;
use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use crate::ImageFormat;
use crate::{
    codec::{EncoderApi, EncoderCfg},
    CodecFormat, I420ToABGR, I420ToARGB, ImageRgb,
};

enum MCColorFormat {
    YUV420P = 0x13,
    NV12 = 0x15,
    Surface = 0x7F000789,
}

/// MediaCodec mime type name
const H264_MIME_TYPE: &str = "video/avc";
const H265_MIME_TYPE: &str = "video/hevc";

pub static H264_DECODER_SUPPORT: AtomicBool = AtomicBool::new(false);
pub static H265_DECODER_SUPPORT: AtomicBool = AtomicBool::new(false);

pub struct MCEncoderConfig {
    pub format: CodecFormat,
    pub width: usize,
    pub height: usize,
    pub color_format: MCColorFormat,
}

pub struct MCEncoder {
    codec: MediaCodec,
}

impl EncoderApi for MCEncoder {
    fn new(cfg: EncoderCfg, i444: bool) -> ResultType<Self>
    where
        Self: Sized,
    {
        let EncoderCfg::MC(cfg) = cfg else {
            bail!("invalid encoder config")
        };
        let mime_type = get_mime_type(cfg.format).ok()?;
        let codec = MediaCodec::from_encoder_type(mime_type)?;
        let media_format = MediaFormat::new();
        media_format.set_str("mime", mime_type);
        media_format.set_i32("width", cfg.width as i32);
        media_format.set_i32("height", cfg.height as i32);
        media_format.set_i32("color-format", cfg.color_format as i32);
        codec.configure(&media_format, None, MediaCodecDirection::Encoder)?;
        codec.start()?;
        return Ok(MCEncoder { codec });
    }

    fn encode_to_message(
        &mut self,
        frame: crate::EncodeInput,
        ms: i64,
    ) -> ResultType<hbb_common::message_proto::VideoFrame> {
        bail!("")
    }

    fn yuvfmt(&self) -> crate::EncodeYuvFormat {
        crate::EncodeYuvFormat {
            pixfmt: crate::Pixfmt::NV12,
            w: 0,
            h: 0,
            stride: vec![],
            u: 0,
            v: 0,
        }
    }

    fn set_quality(&mut self, quality: crate::codec::Quality) -> ResultType<()> {
        Ok(())
    }

    fn bitrate(&self) -> u32 {
        0
    }

    fn support_abr(&self) -> bool {
        false
    }
}

impl MCEncoder {
    pub fn new(cfg: EncoderCfg) -> Option<MCEncoder> {
        let EncoderCfg::MC(cfg) = cfg else {
            return None;
        };
        let mime_type = get_mime_type(cfg.format).ok()?;
        let codec = MediaCodec::from_encoder_type(mime_type)?;
        let media_format = MediaFormat::new();
        media_format.set_str("mime", mime_type);
        media_format.set_i32("width", cfg.width as i32);
        media_format.set_i32("height", cfg.height as i32);
        media_format.set_i32("color-format", cfg.color_format as i32);
        if let Err(e) = codec.configure(&media_format, None, MediaCodecDirection::Encoder) {
            log::error!("Failed to init encoder: {:?}", e);
            return None;
        };
        log::error!("encoder init success");
        if let Err(e) = codec.start() {
            log::error!("Failed to start encoder: {:?}", e);
            return None;
        };
        log::debug!("Init encoder successed!: {:?}", mime_type);
        return Some(MCEncoder { codec });
    }
}

pub struct MediaCodecDecoder {
    decoder: MediaCodec,
    name: String,
}

impl Deref for MediaCodecDecoder {
    type Target = MediaCodec;

    fn deref(&self) -> &Self::Target {
        &self.decoder
    }
}

impl MediaCodecDecoder {
    pub fn new(format: CodecFormat) -> Option<MediaCodecDecoder> {
        match format {
            CodecFormat::H264 => create_media_codec(H264_MIME_TYPE, MediaCodecDirection::Decoder),
            CodecFormat::H265 => create_media_codec(H265_MIME_TYPE, MediaCodecDirection::Decoder),
            _ => {
                log::error!("Unsupported codec format: {}", format);
                None
            }
        }
    }

    // rgb [in/out] fmt and stride must be set in ImageRgb
    pub fn decode(&mut self, data: &[u8], rgb: &mut ImageRgb) -> ResultType<bool> {
        // take dst_stride into account please
        let dst_stride = rgb.stride();
        match self.dequeue_input_buffer(Duration::from_millis(10))? {
            Some(mut input_buffer) => {
                let mut buf = input_buffer.buffer_mut();
                if data.len() > buf.len() {
                    log::error!("Failed to decode, the input data size is bigger than input buf");
                    bail!("The input data size is bigger than input buf");
                }
                buf.write_all(&data)?;
                self.queue_input_buffer(input_buffer, 0, data.len(), 0, 0)?;
            }
            None => {
                log::debug!("Failed to dequeue_input_buffer: No available input_buffer");
            }
        };

        return match self.dequeue_output_buffer(Duration::from_millis(100))? {
            Some(output_buffer) => {
                let res_format = self.output_format();
                let w = res_format
                    .i32("width")
                    .ok_or(Error::msg("Failed to dequeue_output_buffer, width is None"))?
                    as usize;
                let h = res_format.i32("height").ok_or(Error::msg(
                    "Failed to dequeue_output_buffer, height is None",
                ))? as usize;
                let stride = res_format.i32("stride").ok_or(Error::msg(
                    "Failed to dequeue_output_buffer, stride is None",
                ))?;
                let buf = output_buffer.buffer();
                let bps = 4;
                let u = buf.len() * 2 / 3;
                let v = buf.len() * 5 / 6;
                rgb.raw.resize(h * w * bps, 0);
                let y_ptr = buf.as_ptr();
                let u_ptr = buf[u..].as_ptr();
                let v_ptr = buf[v..].as_ptr();
                unsafe {
                    match rgb.fmt() {
                        ImageFormat::ARGB => {
                            I420ToARGB(
                                y_ptr,
                                stride,
                                u_ptr,
                                stride / 2,
                                v_ptr,
                                stride / 2,
                                rgb.raw.as_mut_ptr(),
                                (w * bps) as _,
                                w as _,
                                h as _,
                            );
                        }
                        ImageFormat::ARGB => {
                            I420ToABGR(
                                y_ptr,
                                stride,
                                u_ptr,
                                stride / 2,
                                v_ptr,
                                stride / 2,
                                rgb.raw.as_mut_ptr(),
                                (w * bps) as _,
                                w as _,
                                h as _,
                            );
                        }
                        _ => {
                            bail!("Unsupported image format");
                        }
                    }
                }
                self.release_output_buffer(output_buffer, false)?;
                Ok(true)
            }
            None => {
                log::debug!("Failed to dequeue_output: No available dequeue_output");
                Ok(false)
            }
        };
    }
}

fn get_mime_type(codec: CodecFormat) -> ResultType<&'static str> {
    let mime_type = match codec {
        CodecFormat::VP8 => "video/x-vnd.on2.vp8",
        CodecFormat::VP9 => "video/x-vnd.on2.vp9",
        CodecFormat::AV1 => "video/av01",
        CodecFormat::H264 => "video/avc",
        CodecFormat::H265 => "video/hevc",
        _ => bail("Unsupported codec format: {}", codec),
    };
    Ok(mime_type)
}

fn create_media_codec(name: &str, direction: MediaCodecDirection) -> Option<MediaCodecDecoder> {
    let codec = MediaCodec::from_decoder_type(name)?;
    let media_format = MediaFormat::new();
    media_format.set_str("mime", name);
    media_format.set_i32("width", 0);
    media_format.set_i32("height", 0);
    media_format.set_i32("color-format", 19); // COLOR_FormatYUV420Planar
    if let Err(e) = codec.configure(&media_format, None, direction) {
        log::error!("Failed to init decoder: {:?}", e);
        return None;
    };
    log::error!("decoder init success");
    if let Err(e) = codec.start() {
        log::error!("Failed to start decoder: {:?}", e);
        return None;
    };
    log::debug!("Init decoder successed!: {:?}", name);
    return Some(MediaCodecDecoder {
        decoder: codec,
        name: name.to_owned(),
    });
}

pub fn check_mediacodec() {
    std::thread::spawn(move || {
        // check decoders
        let decoders = MediaCodecDecoder::new_decoders();
        H264_DECODER_SUPPORT.swap(decoders.h264.is_some(), Ordering::SeqCst);
        H265_DECODER_SUPPORT.swap(decoders.h265.is_some(), Ordering::SeqCst);
        decoders.h264.map(|d| d.stop());
        decoders.h265.map(|d| d.stop());
        // TODO encoders
    });
}
