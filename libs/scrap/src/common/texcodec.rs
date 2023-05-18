use std::ffi::c_void;

use crate::{
    codec::{EncoderApi, EncoderCfg},
    CaptureOutputFormat, Frame, ImageFormat, ImageRgb,
};
use hbb_common::{
    allow_err,
    anyhow::{anyhow, bail, Context},
    bytes::Bytes,
    config::HwCodecConfig,
    log,
    message_proto::{EncodedVideoFrame, EncodedVideoFrames, Message, VideoFrame},
    ResultType,
};
use hw_common::{
    DecodeContext, DecodeDriver, DynamicContext, EncodeContext, EncodeDriver, FeatureContext,
    SurfaceFormat, API, MAX_GOP,
};
use texcodec::{
    decode::{DecodeFrame, Decoder},
    encode::{EncodeFrame, Encoder},
};

pub struct TexEncoder {
    encoder: Encoder,
    pub format: hw_common::DataFormat,
}

impl EncoderApi for TexEncoder {
    fn new(cfg: EncoderCfg) -> ResultType<Self>
    where
        Self: Sized,
    {
        match cfg {
            EncoderCfg::TEX(config) => {
                let ctx = EncodeContext {
                    f: FeatureContext {
                        driver: EncodeDriver::MFX,
                        api: API::API_DX11,
                        dataFormat: hw_common::DataFormat::H264,
                    },
                    d: DynamicContext {
                        device: config.device,
                        width: config.width as _,
                        height: config.height as _,
                        kbitrate: config.bitrate,
                        framerate: 30,
                        gop: MAX_GOP as _,
                    },
                };
                match Encoder::new(ctx.clone()) {
                    Ok(encoder) => Ok(TexEncoder {
                        encoder,
                        format: hw_common::DataFormat::H264,
                    }),
                    Err(_) => Err(anyhow!(format!("Failed to create encoder"))),
                }
            }
            _ => Err(anyhow!("encoder type mismatch")),
        }
    }

    fn encode_to_message(
        &mut self,
        frame: Frame,
        _ms: i64,
    ) -> ResultType<hbb_common::message_proto::Message> {
        let texture = frame.texture()?;
        let mut msg_out = Message::new();
        let mut vf = VideoFrame::new();
        let mut frames = Vec::new();
        for frame in self.encode(texture).with_context(|| "Failed to encode")? {
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
            match self.format {
                hw_common::DataFormat::H264 => vf.set_h264s(frames),
                hw_common::DataFormat::H265 => vf.set_h265s(frames),
                _ => bail!("{:?} not supported", self.format),
            }
            msg_out.set_video_frame(vf);
            Ok(msg_out)
        } else {
            Err(anyhow!("no valid frame"))
        }
    }

    fn input_format(&self) -> CaptureOutputFormat {
        CaptureOutputFormat::Texture
    }

    fn set_bitrate(&mut self, bitrate: u32) -> ResultType<()> {
        self.encoder.set_bitrate(bitrate as _).ok();
        Ok(())
    }
}

impl TexEncoder {
    pub fn encode(&mut self, texture: *mut c_void) -> ResultType<Vec<EncodeFrame>> {
        match self.encoder.encode(texture) {
            Ok(v) => {
                let mut data = Vec::<EncodeFrame>::new();
                data.append(v);
                Ok(data)
            }
            Err(_) => Ok(Vec::<EncodeFrame>::new()),
        }
    }
}

pub struct TexDecoder {
    decoder: Decoder,
}

#[derive(Default)]
pub struct TexDecoders {
    pub h264: Option<TexDecoder>,
    pub h265: Option<TexDecoder>,
}

impl TexDecoder {
    pub fn new_decoders(device: *mut c_void) -> TexDecoders {
        let mut h264: Option<TexDecoder> = None;
        let mut h265: Option<TexDecoder> = None;
        if let Ok(decoder) = TexDecoder::new(hw_common::DataFormat::H264, device) {
            h264 = Some(decoder);
        }
        if let Ok(decoder) = TexDecoder::new(hw_common::DataFormat::H265, device) {
            h265 = Some(decoder);
        }
        log::info!(
            "tex new_decoders device: {}, {}, {}",
            device as usize,
            h264.is_some(),
            h265.is_some()
        );
        TexDecoders { h264, h265 }
    }

    pub fn new(dataFormat: hw_common::DataFormat, device: *mut c_void) -> ResultType<Self> {
        let ctx = DecodeContext {
            driver: DecodeDriver::MFX,
            api: API::API_DX11,
            dataFormat,
            outputSurfaceFormat: SurfaceFormat::SURFACE_FORMAT_BGRA,
            hdl: device,
        };
        match Decoder::new(ctx) {
            Ok(decoder) => Ok(Self { decoder }),
            Err(_) => Err(anyhow!(format!("Failed to create decoder"))),
        }
    }
    pub fn decode(&mut self, data: &[u8]) -> ResultType<Vec<TexDecoderImage>> {
        match self.decoder.decode(data) {
            Ok(v) => Ok(v.iter().map(|f| TexDecoderImage { frame: f }).collect()),
            Err(_) => Ok(vec![]),
        }
    }
}

pub struct TexDecoderImage<'a> {
    pub frame: &'a DecodeFrame,
}

impl TexDecoderImage<'_> {}
