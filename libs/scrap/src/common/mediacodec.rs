use crate::{
    android::{
        mediacodec::{FrameImage, VideoDecoderDequeuer, VideoDecoderEnqueuer},
        RelaxedAtomic,
    },
    codec::{EncoderApi, EncoderCfg},
    CodecFormat, I420ToABGR, I420ToARGB, ImageFormat, ImageRgb,
};
use hbb_common::{anyhow::Error, bail, log, ResultType};
use ndk::media::media_codec::{MediaCodec, MediaCodecDirection, MediaFormat};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::{
    io::Write,
    ops::Deref,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

/// MediaCodec mime type name
/// On some models, the vp9 hard decoding effect is better. The hardware decoding is subject to the design mode, that is, the GPU->CPU-GPU rendering method.
pub const H264_MIME_TYPE: &str = "video/avc";
pub const H265_MIME_TYPE: &str = "video/hevc";
pub const VP8_MIME_TYPE: &str = "video/x-vnd.on2.vp8";
pub const VP9_MIME_TYPE: &str = "video/x-vnd.on2.vp9";

// TODO MediaCodecEncoder
pub static H264_DECODER_SUPPORT: AtomicBool = AtomicBool::new(false);
pub static H265_DECODER_SUPPORT: AtomicBool = AtomicBool::new(false);
pub static VP8_DECODER_SUPPORT: AtomicBool = AtomicBool::new(false);
pub static VP9_DECODER_SUPPORT: AtomicBool = AtomicBool::new(false);

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
                log::error!("Unsupported codec format: {:?}", format);
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

fn create_media_codec(name: &str, direction: MediaCodecDirection) -> Option<MediaCodecDecoder> {
    let codec = MediaCodec::from_decoder_type(name)?;
    let media_format = crate::android::mediacodec::configure_media_format(name);

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
        if let Some(h264) = create_media_codec(H264_MIME_TYPE, MediaCodecDirection::Decoder) {
            H264_DECODER_SUPPORT.swap(true, Ordering::SeqCst);
            let _ = h264.stop();
        }
        if let Some(h265) = create_media_codec(H265_MIME_TYPE, MediaCodecDirection::Decoder) {
            H265_DECODER_SUPPORT.swap(true, Ordering::SeqCst);
            let _ = h265.stop();
        }
        if let Some(vp8) = create_media_codec(VP8_MIME_TYPE, MediaCodecDirection::Decoder) {
            VP8_DECODER_SUPPORT.swap(true, Ordering::SeqCst);
            let _ = vp8.stop();
        }
        if let Some(vp9) = create_media_codec(VP9_MIME_TYPE, MediaCodecDirection::Decoder) {
            VP9_DECODER_SUPPORT.swap(true, Ordering::SeqCst);
            let _ = vp9.stop();
        }
        // TODO encoders
    });
}

pub struct XMediaCodecDecoder {
    codec_format: CodecFormat,
    dequeuer: VideoDecoderDequeuer,
    enqueuer: VideoDecoderEnqueuer,
}

impl Drop for XMediaCodecDecoder {
    fn drop(&mut self) {
        self.dequeuer.running.set(false);
    }
}

impl XMediaCodecDecoder {
    pub fn new(codec_format: CodecFormat) -> ResultType<XMediaCodecDecoder> {
        let Ok((enqueuer, dequeuer)) = crate::android::mediacodec::video_decoder_split(
            codec_format,
            MediaCodecDirection::Decoder,
        ) else {
            bail!("video_decoder_split failed");
        };
        Ok(Self {
            codec_format,
            dequeuer,
            enqueuer,
        })
    }

    pub fn decode(
        &mut self,
        data: &[u8],
        rgb: &mut ImageRgb,
        key: &bool,
        pts: &i64,
    ) -> ResultType<bool> {
        let dst_stride = rgb.stride();
        let pts_u64 = if *pts >= 0 { *pts as u64 } else { 0 };
        let flag = if *key { 1 } else { 0 };
        if self
            .enqueuer
            .push_frame_nal(Duration::from_millis(pts_u64), flag, data)
            .ok()
            != Some(true)
        {
            log::debug!("push_frame_nal fail");
        }
        log::info!("push_frame_nal ok");
        if let Some(mut frame_image) = self.dequeuer.dequeue_frame() {
            log::info!("dequeue_frame ok");
            frame_image.i420_to_argb(&mut rgb.raw);
            return Ok(true);
        }
        Ok(false)
    }
}
