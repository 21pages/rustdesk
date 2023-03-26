use crate::{
    codec::{EncoderApi, EncoderCfg},
    hw, ImageFormat, HW_STRIDE_ALIGN,
};
use codec_common::{DataFormat, DecodeContext, EncodeContext, PixelFormat};
use hbb_common::{
    anyhow::{anyhow, bail, Context},
    bytes::Bytes,
    config::HwCodecConfig,
    lazy_static, log,
    message_proto::{EncodedVideoFrame, EncodedVideoFrames, Message, VideoFrame},
    ResultType,
};
use hwcodec::{
    decode::{self, DecodeFrame, Decoder},
    encode::{self, EncodeFrame, Encoder},
};
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    static ref HW_ENCODER: Arc<Mutex<Option<EncodeContext>>> = Default::default();
}

const CFG_KEY_ENCODER: &str = "bestHwEncoders";
const CFG_KEY_DECODER: &str = "bestHwDecoders";

pub struct HwEncoder {
    encoder: Encoder,
    ctx: EncodeContext,
    yuv: Vec<u8>,
}

impl EncoderApi for HwEncoder {
    fn new(cfg: EncoderCfg) -> ResultType<Self>
    where
        Self: Sized,
    {
        match cfg {
            EncoderCfg::HW(ctx) => match Encoder::new(ctx.clone()) {
                Ok(encoder) => Ok(HwEncoder {
                    encoder,
                    ctx,
                    yuv: vec![],
                }),
                Err(_) => Err(anyhow!(format!("Failed to create encoder"))),
            },
            _ => Err(anyhow!("encoder type mismatch")),
        }
    }

    fn encode_to_message(
        &mut self,
        frame: &[u8],
        _ms: i64,
    ) -> ResultType<hbb_common::message_proto::Message> {
        let mut msg_out = Message::new();
        let mut vf = VideoFrame::new();
        let mut frames = Vec::new();
        for frame in self.encode(frame).with_context(|| "Failed to encode")? {
            frames.push(EncodedVideoFrame {
                data: Bytes::from(frame.data),
                pts: frame.pts as _,
                key: frame.key == 1,
                ..Default::default()
            });
        }
        if frames.len() > 0 {
            let frames = EncodedVideoFrames {
                frames: frames.into(),
                ..Default::default()
            };
            match self.ctx.dataFormat {
                DataFormat::H264 => vf.set_h264s(frames),
                DataFormat::H265 => vf.set_h265s(frames),
                _ => bail!("unsupported data format: {:?}", self.ctx.dataFormat),
            }
            msg_out.set_video_frame(vf);
            Ok(msg_out)
        } else {
            Err(anyhow!("no valid frame"))
        }
    }

    fn use_yuv(&self) -> bool {
        false
    }

    fn set_bitrate(&mut self, bitrate: u32) -> ResultType<()> {
        // self.encoder.set_bitrate((bitrate * 1000) as _).ok();
        Ok(())
    }
}

impl HwEncoder {
    pub fn best() -> encode::Best {
        get_encode_config().unwrap_or(encode::Best {
            h264: None,
            h265: None,
        })
    }

    pub fn current() -> Arc<Mutex<Option<EncodeContext>>> {
        HW_ENCODER.clone()
    }

    pub fn encode(&mut self, bgra: &[u8]) -> ResultType<Vec<EncodeFrame>> {
        let (linesizes, offsets, length) =
            hw::linesize_offset_length(self.ctx.pixfmt, self.ctx.width, self.ctx.height);
        match self.ctx.pixfmt {
            PixelFormat::NV12 => hw::hw_bgra_to_nv12(
                self.encoder.ctx.width as _,
                self.encoder.ctx.height as _,
                &linesizes,
                &offsets,
                length,
                bgra,
                &mut self.yuv,
            ),
            _ => bail!("unsupported pixfmt: {:?}", self.ctx.pixfmt),
        }
        let (yuv, linesizes) =
            hw::split_yuv(self.ctx.pixfmt, self.ctx.width, self.ctx.height, &self.yuv);
        match self.encoder.encode(yuv, linesizes) {
            Ok(v) => {
                let mut data = Vec::<EncodeFrame>::new();
                data.append(v);
                Ok(data)
            }
            Err(_) => Ok(Vec::<EncodeFrame>::new()),
        }
    }
}

pub struct HwDecoder {
    decoder: Decoder,
    pub ctx: DecodeContext,
}

#[derive(Default)]
pub struct HwDecoders {
    pub h264: Option<HwDecoder>,
    pub h265: Option<HwDecoder>,
}

impl HwDecoder {
    pub fn best() -> decode::Best {
        get_decode_config().unwrap_or(decode::Best {
            h264: None,
            h265: None,
        })
    }

    pub fn new_decoders() -> HwDecoders {
        let best = HwDecoder::best();
        let mut h264: Option<HwDecoder> = None;
        let mut h265: Option<HwDecoder> = None;
        let mut fail = false;

        if let Some(ctx) = best.h264 {
            h264 = HwDecoder::new(ctx).ok();
            if h264.is_none() {
                fail = true;
            }
        }
        if let Some(ctx) = best.h265 {
            h265 = HwDecoder::new(ctx).ok();
            if h265.is_none() {
                fail = true;
            }
        }
        if fail {
            check_config_process(true);
        }
        HwDecoders { h264, h265 }
    }

    pub fn new(ctx: DecodeContext) -> ResultType<Self> {
        match Decoder::new(ctx.clone()) {
            Ok(decoder) => Ok(HwDecoder { decoder, ctx }),
            Err(_) => Err(anyhow!(format!("Failed to create decoder"))),
        }
    }
    pub fn decode(&mut self, data: &[u8]) -> ResultType<Vec<HwDecoderImage>> {
        match self.decoder.decode(data) {
            Ok(v) => Ok(v.iter().map(|f| HwDecoderImage { frame: f }).collect()),
            Err(_) => Ok(vec![]),
        }
    }
}

pub struct HwDecoderImage<'a> {
    frame: &'a DecodeFrame,
}

impl HwDecoderImage<'_> {
    // take dst_stride into account when you convert
    pub fn to_fmt(
        &self,
        (fmt, dst_stride): (ImageFormat, usize),
        fmt_data: &mut Vec<u8>,
        i420: &mut Vec<u8>,
    ) -> ResultType<()> {
        let frame = self.frame;
        match frame.pixfmt {
            PixelFormat::NV12 => hw::hw_nv12_to(
                fmt,
                frame.width,
                frame.height,
                &frame.data[0],
                &frame.data[1],
                frame.linesize[0],
                frame.linesize[1],
                fmt_data,
                i420,
                HW_STRIDE_ALIGN as i32,
            ),
            PixelFormat::I420 => {
                hw::hw_i420_to(
                    fmt,
                    frame.width as _,
                    frame.height as _,
                    frame.data[0].as_ptr(),
                    frame.data[1].as_ptr(),
                    frame.data[2].as_ptr(),
                    frame.linesize[0] as _,
                    frame.linesize[1] as _,
                    frame.linesize[2] as _,
                    fmt_data,
                );
                return Ok(());
            }
        }
    }

    pub fn bgra(&self, bgra: &mut Vec<u8>, i420: &mut Vec<u8>) -> ResultType<()> {
        self.to_fmt((ImageFormat::ARGB, 1), bgra, i420)
    }

    pub fn rgba(&self, rgba: &mut Vec<u8>, i420: &mut Vec<u8>) -> ResultType<()> {
        self.to_fmt((ImageFormat::ABGR, 1), rgba, i420)
    }
}

fn get_encode_config() -> ResultType<encode::Best> {
    let k = CFG_KEY_ENCODER;
    let v = HwCodecConfig::get()
        .options
        .get(k)
        .unwrap_or(&"".to_owned())
        .to_owned();
    match encode::Best::deserialize(&v) {
        Ok(v) => Ok(v),
        Err(_) => Err(anyhow!("Failed to get config:{}", k)),
    }
}

fn get_decode_config() -> ResultType<decode::Best> {
    let k = CFG_KEY_DECODER;
    let v = HwCodecConfig::get()
        .options
        .get(k)
        .unwrap_or(&"".to_owned())
        .to_owned();
    match decode::Best::deserialize(&v) {
        Ok(v) => Ok(v),
        Err(_) => Err(anyhow!("Failed to get config:{}", k)),
    }
}

pub fn check_config() {
    let encoders = encode::Best::new(encode::available());
    let decoders = decode::Best::new(decode::available());

    if let Ok(old_encoders) = get_encode_config() {
        if let Ok(old_decoders) = get_decode_config() {
            if encoders == old_encoders && decoders == old_decoders {
                return;
            }
        }
    }

    if let Ok(encoders) = encoders.serialize() {
        if let Ok(decoders) = decoders.serialize() {
            let mut config = HwCodecConfig::load();
            config.options.insert(CFG_KEY_ENCODER.to_owned(), encoders);
            config.options.insert(CFG_KEY_DECODER.to_owned(), decoders);
            config.store();
            return;
        }
    }
    log::error!("Failed to serialize codec info");
}

pub fn check_config_process(force_reset: bool) {
    use hbb_common::sysinfo::{ProcessExt, System, SystemExt};

    std::thread::spawn(move || {
        if force_reset {
            HwCodecConfig::remove();
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(file_name) = exe.file_name().to_owned() {
                let s = System::new_all();
                let arg = "--check-hwcodec-config";
                for process in s.processes_by_name(&file_name.to_string_lossy().to_string()) {
                    if process.cmd().iter().any(|cmd| cmd.contains(arg)) {
                        log::warn!("already have process {}", arg);
                        return;
                    }
                }
                if let Ok(mut child) = std::process::Command::new(exe).arg(arg).spawn() {
                    let second = 3;
                    std::thread::sleep(std::time::Duration::from_secs(second));
                    // kill: Different platforms have different results
                    child.kill().ok();
                    HwCodecConfig::refresh();
                }
            }
        };
    });
}
