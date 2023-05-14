use std::{
    ffi::c_void,
    sync::{Arc, Mutex},
};

use crate::{
    codec::{base_bitrate, EncoderApi, EncoderCfg},
    AdapterDevice, CaptureOutputFormat, CodecName, Frame,
};
use gpu_video_codec::gvc_common::{
    self, Available, DecodeContext, DynamicContext, EncodeContext, FeatureContext, MAX_GOP,
};
use gpu_video_codec::{
    decode::{self, DecodeFrame, Decoder},
    encode::{self, EncodeFrame, Encoder},
};
use hbb_common::{
    allow_err,
    anyhow::{anyhow, bail, Context},
    bytes::Bytes,
    log,
    message_proto::{EncodedVideoFrame, EncodedVideoFrames, Message, VideoFrame},
    ResultType,
};

const OUTPUT_SHARED_HANDLE: bool = false;

lazy_static::lazy_static! {
    static ref VIDEO_SERVICE_ADAPTER_LUID: Arc<Mutex<Option<i64>>> = Default::default();
    static ref VIDEO_SERVICE_NOT_USE: Arc<Mutex<bool>> = Default::default();
    static ref THIS_PROCESS_ENCODING_NOT_USE: Arc<Mutex<bool>> = Default::default();
}

pub struct GvcEncoder {
    encoder: Encoder,
    pub format: gvc_common::DataFormat,
    ctx: EncodeContext,
    bitrate: u32,
    last_bad_len: usize,
    same_bad_len_counter: usize,
}

impl EncoderApi for GvcEncoder {
    fn new(cfg: EncoderCfg) -> ResultType<Self>
    where
        Self: Sized,
    {
        match cfg {
            EncoderCfg::GVC(config) => {
                let b = Self::convert_quality(config.quality);
                let base_bitrate = base_bitrate(config.width as _, config.height as _);
                let mut bitrate = base_bitrate * b / 100;
                if base_bitrate <= 0 {
                    bitrate = base_bitrate;
                }
                let gop = config.keyframe_interval.unwrap_or(MAX_GOP as _) as i32;
                let ctx = EncodeContext {
                    f: config.feature.clone(),
                    d: DynamicContext {
                        device: Some(config.device.device),
                        width: config.width as _,
                        height: config.height as _,
                        kbitrate: bitrate as _,
                        framerate: 30,
                        gop,
                    },
                };
                match Encoder::new(ctx.clone()) {
                    Ok(encoder) => Ok(GvcEncoder {
                        encoder,
                        ctx,
                        format: config.feature.data_format,
                        bitrate,
                        last_bad_len: 0,
                        same_bad_len_counter: 0,
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
            // println!("encode tex: {:?}, len:{}", texture, frame.data.len());
            frames.push(EncodedVideoFrame {
                data: Bytes::from(frame.data),
                pts: frame.pts as _,
                key: frame.key == 1,
                ..Default::default()
            });
        }
        if frames.len() > 0 {
            // This kind of problem has occurred. After a period of time when using AMD encoding,
            // the encoding length is fixed at about 40, and the picture is still
            const MIN_BAD_LEN: usize = 100;
            const MAX_BAD_COUNTER: usize = 10;
            let first_frame_len = frames[0].data.len();
            if first_frame_len < MIN_BAD_LEN {
                if first_frame_len == self.last_bad_len {
                    self.same_bad_len_counter += 1;
                    if self.same_bad_len_counter >= MAX_BAD_COUNTER {
                        *THIS_PROCESS_ENCODING_NOT_USE.lock().unwrap() = true;
                        log::info!(
                            "{} times encoding len is {}",
                            self.same_bad_len_counter,
                            self.last_bad_len
                        );
                        bail!(crate::codec::ENCODE_NEED_SWITCH);
                    }
                } else {
                    self.last_bad_len = first_frame_len;
                    self.same_bad_len_counter = 0;
                }
            }
            let frames = EncodedVideoFrames {
                frames: frames.into(),
                ..Default::default()
            };
            match self.format {
                gvc_common::DataFormat::H264 => vf.set_h264s(frames),
                gvc_common::DataFormat::H265 => vf.set_h265s(frames),
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

    fn set_quality(&mut self, quality: crate::codec::Quality) -> ResultType<()> {
        let b = Self::convert_quality(quality);
        let bitrate = base_bitrate(self.ctx.d.width as _, self.ctx.d.height as _) * b / 100;
        if bitrate > 0 {
            self.encoder.set_bitrate((bitrate) as _).ok();
            self.bitrate = bitrate;
        }
        Ok(())
    }

    fn bitrate(&self) -> u32 {
        self.bitrate
    }
}

impl GvcEncoder {
    pub fn try_get(device: &AdapterDevice, name: CodecName) -> Option<FeatureContext> {
        let v: Vec<_> = Self::possible_available(name)
            .drain(..)
            .filter(|e| e.luid == device.luid)
            .collect();
        if v.len() > 0 {
            Some(v[0].clone())
        } else {
            None
        }
    }

    pub fn possible_available(name: CodecName) -> Vec<FeatureContext> {
        if VIDEO_SERVICE_NOT_USE.lock().unwrap().clone()
            || THIS_PROCESS_ENCODING_NOT_USE.lock().unwrap().clone()
        {
            return vec![];
        }
        let data_format = match name {
            CodecName::H264G => gvc_common::DataFormat::H264,
            CodecName::H265G => gvc_common::DataFormat::H265,
            _ => return vec![],
        };
        let mut v: Vec<_> = get_available_config()
            .map(|c| c.e)
            .unwrap_or_default()
            .drain(..)
            .filter(|c| c.data_format == data_format)
            .collect();
        let luid = VIDEO_SERVICE_ADAPTER_LUID
            .lock()
            .unwrap()
            .unwrap_or_default();
        if luid != 0 {
            v.retain(|e| e.luid == luid);
        }
        v
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

    pub fn convert_quality(quality: crate::codec::Quality) -> u32 {
        use crate::codec::Quality;
        match quality {
            Quality::Best => 150,
            Quality::Balanced => 100,
            Quality::Low => 50,
            Quality::Custom(b) => b,
        }
    }

    pub fn set_video_service_adapter_luid(luid: Option<i64>) {
        *VIDEO_SERVICE_ADAPTER_LUID.lock().unwrap() = luid;
    }

    pub fn set_video_service_not_use(not_use: bool) {
        *VIDEO_SERVICE_NOT_USE.lock().unwrap() = not_use;
    }

    pub fn set_this_process_not_use(not_use: bool) {
        *THIS_PROCESS_ENCODING_NOT_USE.lock().unwrap() = not_use;
    }
}

pub struct GvcDecoder {
    decoder: Decoder,
}

#[derive(Default)]
pub struct GvcDecoders {
    pub h264: Option<GvcDecoder>,
    pub h265: Option<GvcDecoder>,
}

impl GvcDecoder {
    pub fn try_get(name: CodecName, luid: i64) -> Option<DecodeContext> {
        let v: Vec<_> = Self::possible_available(name, Some(luid));
        if v.len() > 0 {
            Some(v[0].clone())
        } else {
            None
        }
    }

    pub fn possible_available(name: CodecName, luid: Option<i64>) -> Vec<DecodeContext> {
        let data_format = match name {
            CodecName::H264G => gvc_common::DataFormat::H264,
            CodecName::H265G => gvc_common::DataFormat::H265,
            _ => return vec![],
        };
        let mut v: Vec<_> = get_available_config()
            .map(|c| c.d)
            .unwrap_or_default()
            .drain(..)
            .filter(|c| c.data_format == data_format)
            .collect();
        let luid = luid.unwrap_or_default();
        if luid != 0 {
            v.retain(|d| d.luid == luid);
        }
        v
    }

    pub fn new_decoders(luid: i64) -> GvcDecoders {
        let mut h264: Option<GvcDecoder> = None;
        let mut h265: Option<GvcDecoder> = None;
        if let Ok(decoder) = GvcDecoder::new(CodecName::H264G, luid) {
            h264 = Some(decoder);
        }
        if let Ok(decoder) = GvcDecoder::new(CodecName::H265G, luid) {
            h265 = Some(decoder);
        }
        log::info!(
            "tex new_decoders device: {}, {}",
            h264.is_some(),
            h265.is_some()
        );
        GvcDecoders { h264, h265 }
    }

    pub fn new(name: CodecName, luid: i64) -> ResultType<Self> {
        let ctx = Self::try_get(name, luid).ok_or(anyhow!("Failed to get decode context"))?;
        match Decoder::new(ctx) {
            Ok(decoder) => Ok(Self { decoder }),
            Err(_) => Err(anyhow!(format!("Failed to create decoder"))),
        }
    }
    pub fn decode(&mut self, data: &[u8]) -> ResultType<Vec<GvcDecoderImage>> {
        match self.decoder.decode(data) {
            Ok(v) => Ok(v.iter().map(|f| GvcDecoderImage { frame: f }).collect()),
            Err(_) => Ok(vec![]),
        }
    }
}

pub struct GvcDecoderImage<'a> {
    pub frame: &'a DecodeFrame,
}

impl GvcDecoderImage<'_> {}

fn get_available_config() -> ResultType<Available> {
    let available = hbb_common::config::GpuVideoCodecConfig::load().available;
    match Available::deserialize(&available) {
        Ok(v) => Ok(v),
        Err(_) => Err(anyhow!("Failed to deserialize:{}", available)),
    }
}

pub fn check_available_gpu_video_codec() {
    let d = DynamicContext {
        device: None,
        width: 1920,
        height: 1080,
        kbitrate: 5000,
        framerate: 60,
        gop: MAX_GOP as _,
    };
    let encoders = encode::available(d);
    let decoders = decode::available(OUTPUT_SHARED_HANDLE);
    let available = Available {
        e: encoders,
        d: decoders,
    };

    if let Ok(available) = available.serialize() {
        let mut config = hbb_common::config::GpuVideoCodecConfig::load();
        config.available = available;
        config.store();
        return;
    }
    log::error!("Failed to serialize gpu_video_codec");
}

pub fn gpu_video_codec_new_check_process() {
    use std::sync::Once;

    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::thread::spawn(move || {
            // Remove to avoid checking process errors
            // But when the program is just started, the configuration file has not been updated, and the new connection will read an empty configuration
            hbb_common::config::GpuVideoCodecConfig::clear();
            if let Ok(exe) = std::env::current_exe() {
                    let arg = "--check-gpu-video-codec-config";
                    if let Ok(mut child) = std::process::Command::new(exe).arg(arg).spawn() {
                        // wait up to 10 seconds
                        for _ in 0..10 {
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            if let Ok(Some(_)) = child.try_wait() {
                                break;
                            }
                        }
                        allow_err!(child.kill());
                        std::thread::sleep(std::time::Duration::from_millis(30));
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                log::info!("Check gpu_video_codec config, exit with: {status}")
                            }
                            Ok(None) => {
                                log::info!(
                                    "Check gpu_video_codec config, status not ready yet, let's really wait"
                                );
                                let res = child.wait();
                                log::info!("Check gpu_video_codec config, wait result: {res:?}");
                            }
                            Err(e) => {
                                log::error!(
                                    "Check gpu_video_codec config, error attempting to wait: {e}"
                                )
                            }
                        }
                    }
            };
        });
    });
}
