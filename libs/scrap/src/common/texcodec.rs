use std::ffi::c_void;

use crate::{
    codec::{EncoderApi, EncoderCfg},
    dxgi::device_to_adapter_device,
    AdapterDevice, CaptureOutputFormat, CodecName, Frame,
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
    Available, DecodeContext, DecodeDriver, DynamicContext, EncodeContext, EncodeDriver,
    FeatureContext, SurfaceFormat, API, MAX_GOP,
};
use texcodec::{
    decode::{self, DecodeFrame, Decoder},
    encode::{self, EncodeFrame, Encoder},
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
                    f: config.feature.clone(),
                    d: DynamicContext {
                        device: Some(config.device.device),
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
                        format: config.feature.data_format,
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
            println!("encode tex: {:?}, len:{}", texture, frame.data.len());
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
    pub fn try_get(device: &AdapterDevice, name: CodecName) -> Option<FeatureContext> {
        let data_format = match name {
            CodecName::H264(_) => hw_common::DataFormat::H264,
            CodecName::H265(_) => hw_common::DataFormat::H265,
            _ => return None,
        };
        let v: Vec<_> = get_available_config()
            .map(|c| c.e)
            .unwrap_or_default()
            .drain(..)
            .filter(|c| c.luid == device.luid && c.data_format == data_format)
            .collect();
        if v.len() > 0 {
            Some(v[0].clone())
        } else {
            None
        }
    }

    pub fn possible_available(name: CodecName) -> Vec<FeatureContext> {
        let data_format = match name {
            CodecName::H264(_) => hw_common::DataFormat::H264,
            CodecName::H265(_) => hw_common::DataFormat::H265,
            _ => return vec![],
        };
        get_available_config()
            .map(|c| c.e)
            .unwrap_or_default()
            .drain(..)
            .filter(|c| c.data_format == data_format)
            .collect()
    }

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
    pub fn try_get(luid: i64, data_format: hw_common::DataFormat) -> Option<DecodeContext> {
        let v: Vec<_> = get_available_config()
            .map(|c| c.d)
            .unwrap_or_default()
            .drain(..)
            .filter(|c| c.luid == luid && c.data_format == data_format)
            .collect();
        if v.len() > 0 {
            Some(v[0].clone())
        } else {
            None
        }
    }

    pub fn possible_available(name: CodecName) -> Vec<DecodeContext> {
        let data_format = match name {
            CodecName::H264(_) => hw_common::DataFormat::H264,
            CodecName::H265(_) => hw_common::DataFormat::H265,
            _ => return vec![],
        };
        get_available_config()
            .map(|c| c.d)
            .unwrap_or_default()
            .drain(..)
            .filter(|c| c.data_format == data_format)
            .collect()
    }

    pub fn new_decoders(luid: i64) -> TexDecoders {
        let mut h264: Option<TexDecoder> = None;
        let mut h265: Option<TexDecoder> = None;
        if let Ok(decoder) = TexDecoder::new(hw_common::DataFormat::H264, luid) {
            h264 = Some(decoder);
        }
        if let Ok(decoder) = TexDecoder::new(hw_common::DataFormat::H265, luid) {
            h265 = Some(decoder);
        }
        log::info!(
            "tex new_decoders device: {}, {}",
            h264.is_some(),
            h265.is_some()
        );
        TexDecoders { h264, h265 }
    }

    pub fn new(data_format: hw_common::DataFormat, luid: i64) -> ResultType<Self> {
        let ctx =
            Self::try_get(luid, data_format).ok_or(anyhow!("Failed to get decode context"))?;
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

fn get_available_config() -> ResultType<Available> {
    let available = hbb_common::config::TexCodecConfig::load().available;
    match Available::deserialize(&available) {
        Ok(v) => Ok(v),
        Err(_) => Err(anyhow!("Failed to deserialize:{}", available)),
    }
}

pub fn check_available_texcodec() {
    let d = DynamicContext {
        device: None,
        width: 1920,
        height: 1080,
        kbitrate: 5000,
        framerate: 60,
        gop: MAX_GOP as _,
    };
    let encoders = encode::available(d);
    let decoders = decode::available();
    let available = Available {
        e: encoders,
        d: decoders,
    };

    if let Ok(available) = available.serialize() {
        let mut config = hbb_common::config::TexCodecConfig::load();
        config.available = available;
        config.store();
        return;
    }
    log::error!("Failed to serialize texcodec");
}

pub fn texcodec_new_check_process() {
    use hbb_common::sysinfo::{ProcessExt, System, SystemExt};

    std::thread::spawn(move || {
        // Remove to avoid checking process errors
        // But when the program is just started, the configuration file has not been updated, and the new connection will read an empty configuration
        hbb_common::config::TexCodecConfig::remove();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(file_name) = exe.file_name().to_owned() {
                let s = System::new_all();
                let arg = "--check-texcodec-config";
                for process in s.processes_by_name(&file_name.to_string_lossy().to_string()) {
                    if process.cmd().iter().any(|cmd| cmd.contains(arg)) {
                        log::warn!("already have process {}", arg);
                        return;
                    }
                }
                if let Ok(mut child) = std::process::Command::new(exe).arg(arg).spawn() {
                    // wait up to 10 seconds
                    for _ in 0..10 {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        if let Ok(Some(status)) = child.try_wait() {
                            break;
                        }
                    }
                    allow_err!(child.kill());
                    std::thread::sleep(std::time::Duration::from_millis(30));
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            log::info!("Check texcodec config, exit with: {status}")
                        }
                        Ok(None) => {
                            log::info!(
                                "Check texcodec config, status not ready yet, let's really wait"
                            );
                            let res = child.wait();
                            log::info!("Check texcodec config, wait result: {res:?}");
                        }
                        Err(e) => {
                            log::error!("Check texcodec config, error attempting to wait: {e}")
                        }
                    }
                }
            }
        };
    });
}
