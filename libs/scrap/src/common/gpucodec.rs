use std::{
    ffi::c_void,
    sync::{Arc, Mutex},
};

use crate::{
    codec::{base_bitrate, EncoderApi, EncoderCfg},
    AdapterDevice, CaptureOutputFormat, CodecName, Frame,
};
use gpucodec::gpu_common::{
    self, Available, DecodeContext, DynamicContext, EncodeContext, FeatureContext, MAX_GOP,
};
use gpucodec::{
    decode::{self, DecodeFrame, Decoder},
    encode::{self, EncodeFrame, Encoder},
};
use hbb_common::{
    allow_err,
    anyhow::{anyhow, bail, Context},
    bytes::Bytes,
    log,
    message_proto::{EncodedVideoFrame, EncodedVideoFrames, VideoFrame},
    ResultType,
};

const OUTPUT_SHARED_HANDLE: bool = false;

// https://www.reddit.com/r/buildapc/comments/d2m4ny/two_graphics_cards_two_monitors/
// https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12device-getadapterluid#remarks
lazy_static::lazy_static! {
    static ref VIDEO_SERVICE_ADAPTER_LUID: Arc<Mutex<Option<i64>>> = Default::default(); // different display
    static ref THIS_TIME_NOT_USE: Arc<Mutex<bool>> = Default::default();
}

pub struct GpuEncoder {
    encoder: Encoder,
    pub format: gpu_common::DataFormat,
    ctx: EncodeContext,
    bitrate: u32,
    last_frame_len: usize,
    same_bad_len_counter: usize,
}

impl EncoderApi for GpuEncoder {
    fn new(cfg: EncoderCfg) -> ResultType<Self>
    where
        Self: Sized,
    {
        match cfg {
            EncoderCfg::GPU(config) => {
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
                    Ok(encoder) => Ok(GpuEncoder {
                        encoder,
                        ctx,
                        format: config.feature.data_format,
                        bitrate,
                        last_frame_len: 0,
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
    ) -> ResultType<hbb_common::message_proto::VideoFrame> {
        let texture = frame.texture()?;
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
            // This kind of problem is occurred after a period of time when using AMD encoding,
            // the encoding length is fixed at about 40, and the picture is still
            const MIN_BAD_LEN: usize = 100;
            const MAX_BAD_COUNTER: usize = 30;
            let this_frame_len = frames[0].data.len();
            if this_frame_len < MIN_BAD_LEN && this_frame_len == self.last_frame_len {
                self.same_bad_len_counter += 1;
                if self.same_bad_len_counter >= MAX_BAD_COUNTER {
                    log::info!(
                        "{} times encoding len is {}, switch",
                        self.same_bad_len_counter,
                        self.last_frame_len
                    );
                    bail!(crate::codec::ENCODE_NEED_SWITCH);
                }
            } else {
                self.same_bad_len_counter = 0;
            }
            self.last_frame_len = this_frame_len;
            let frames = EncodedVideoFrames {
                frames: frames.into(),
                ..Default::default()
            };
            match self.format {
                gpu_common::DataFormat::H264 => vf.set_h264s(frames),
                gpu_common::DataFormat::H265 => vf.set_h265s(frames),
                _ => bail!("{:?} not supported", self.format),
            }
            Ok(vf)
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

impl GpuEncoder {
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
        let not_use = THIS_TIME_NOT_USE.lock().unwrap().clone();
        if not_use {
            log::info!("currently not use gpucodec encoders");
            return vec![];
        }
        let data_format = match name {
            CodecName::H264GPU => gpu_common::DataFormat::H264,
            CodecName::H265GPU => gpu_common::DataFormat::H265,
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
        log::info!("VIDEO_SERVICE_ADAPTER_LUID:{luid}");
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
        log::info!("set_video_service_adapter_luid:{:?}", luid);
        *VIDEO_SERVICE_ADAPTER_LUID.lock().unwrap() = luid;
    }

    pub fn set_this_time_not_use(not_use: bool) {
        log::info!("set_video_service_not_use:{not_use}");
        *THIS_TIME_NOT_USE.lock().unwrap() = not_use;
    }
}

pub struct GpuDecoder {
    decoder: Decoder,
}

#[derive(Default)]
pub struct GpuDecoders {
    pub h264: Option<GpuDecoder>,
    pub h265: Option<GpuDecoder>,
}

impl GpuDecoder {
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
            CodecName::H264GPU => gpu_common::DataFormat::H264,
            CodecName::H265GPU => gpu_common::DataFormat::H265,
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

    pub fn new_decoders(luid: i64) -> GpuDecoders {
        let mut h264: Option<GpuDecoder> = None;
        let mut h265: Option<GpuDecoder> = None;
        if let Ok(decoder) = GpuDecoder::new(CodecName::H264GPU, luid) {
            h264 = Some(decoder);
        }
        if let Ok(decoder) = GpuDecoder::new(CodecName::H265GPU, luid) {
            h265 = Some(decoder);
        }
        log::info!(
            "tex new_decoders device: {}, {}",
            h264.is_some(),
            h265.is_some()
        );
        GpuDecoders { h264, h265 }
    }

    pub fn new(name: CodecName, luid: i64) -> ResultType<Self> {
        let ctx = Self::try_get(name, luid).ok_or(anyhow!("Failed to get decode context"))?;
        match Decoder::new(ctx) {
            Ok(decoder) => Ok(Self { decoder }),
            Err(_) => Err(anyhow!(format!("Failed to create decoder"))),
        }
    }
    pub fn decode(&mut self, data: &[u8]) -> ResultType<Vec<GpuDecoderImage>> {
        match self.decoder.decode(data) {
            Ok(v) => Ok(v.iter().map(|f| GpuDecoderImage { frame: f }).collect()),
            Err(e) => Err(anyhow!(e)),
        }
    }
}

pub struct GpuDecoderImage<'a> {
    pub frame: &'a DecodeFrame,
}

impl GpuDecoderImage<'_> {}

fn get_available_config() -> ResultType<Available> {
    let available = hbb_common::config::GpucodecConfig::load().available;
    match Available::deserialize(&available) {
        Ok(v) => Ok(v),
        Err(_) => Err(anyhow!("Failed to deserialize:{}", available)),
    }
}

pub fn check_available_gpucodec() {
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
        let mut config = hbb_common::config::GpucodecConfig::load();
        config.available = available;
        config.store();
        return;
    }
    log::error!("Failed to serialize gpucodec");
}

pub fn gpucodec_new_check_process() {
    use std::sync::Once;

    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::thread::spawn(move || {
            // Remove to avoid checking process errors
            // But when the program is just started, the configuration file has not been updated, and the new connection will read an empty configuration
            hbb_common::config::GpucodecConfig::clear();
            if let Ok(exe) = std::env::current_exe() {
                let arg = "--check-gpucodec-config";
                if let Ok(mut child) = std::process::Command::new(exe).arg(arg).spawn() {
                    // wait up to 30 seconds
                    for _ in 0..30 {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        if let Ok(Some(_)) = child.try_wait() {
                            break;
                        }
                    }
                    allow_err!(child.kill());
                    std::thread::sleep(std::time::Duration::from_millis(30));
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            log::info!("Check gpucodec config, exit with: {status}")
                        }
                        Ok(None) => {
                            log::info!(
                                "Check gpucodec config, status not ready yet, let's really wait"
                            );
                            let res = child.wait();
                            log::info!("Check gpucodec config, wait result: {res:?}");
                        }
                        Err(e) => {
                            log::error!("Check gpucodec config, error attempting to wait: {e}")
                        }
                    }
                }
            };
        });
    });
}
