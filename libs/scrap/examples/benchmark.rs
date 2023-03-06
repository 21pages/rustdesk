use docopt::Docopt;
use hbb_common::env_logger::{init_from_env, Env, DEFAULT_FILTER_ENV};
use scrap::{
    bgra_to_i420,
    codec::{EncoderApi, EncoderCfg},
    Capturer, Display, TraitCapturer, VpxDecoder, VpxDecoderConfig, VpxEncoder, VpxEncoderConfig,
    VpxVideoCodecId, STRIDE_ALIGN,
};
use std::{io::Write, time::Instant};

// cargo run --package scrap --example benchmark --release --features hwcodec

const USAGE: &'static str = "
Codec benchmark.

Usage:
  benchmark [--count=COUNT] [--bitrate=KBS] [--hw-pixfmt=PIXFMT]
  benchmark (-h | --help)

Options:
  -h --help             Show this screen.
  --count=COUNT         Capture frame count [default: 100].
  --bitrate=KBS         Video bitrate in kilobits per second [default: 5000].
  --hw-pixfmt=PIXFMT    Hardware codec pixfmt. [default: i420]
                        Valid values: i420, nv12.
";

#[derive(Debug, serde::Deserialize)]
struct Args {
    flag_count: usize,
    flag_bitrate: usize,
    flag_hw_pixfmt: Pixfmt,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) enum Pixfmt {
    I420,
    NV12,
}

fn main() {
    init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let bitrate_k = args.flag_bitrate;
    let count = args.flag_count;
    let (bgras, width, height) = capture_bgra(count);
    println!(
        "benchmark {}x{} bitrate:{}k hw_pixfmt:{:?}",
        width, height, bitrate_k, args.flag_hw_pixfmt
    );
    test_vp9(&bgras, width, height, bitrate_k, count);
    #[cfg(feature = "hwcodec")]
    hw::test(bgras, width, height, bitrate_k, count, args.flag_hw_pixfmt);
}

fn capture_bgra(count: usize) -> (Vec<Vec<u8>>, usize, usize) {
    let mut index = 0;
    let mut displays = Display::all().unwrap();
    for i in 0..displays.len() {
        if displays[i].is_primary() {
            index = i;
            break;
        }
    }
    let d = displays.remove(index);
    let mut c = Capturer::new(d, false).unwrap();
    let mut v = vec![];
    loop {
        if let Ok(frame) = c.frame(std::time::Duration::from_secs_f32(1. / 30.)) {
            let bgra = frame.0.to_vec();
            v.push(bgra);
            print!("\rcapture {}/{}", v.len(), count);
            std::io::stdout().flush().ok();
            if v.len() == count {
                println!();
                return (v, c.width(), c.height());
            }
            std::thread::sleep(std::time::Duration::from_secs_f32(1. / 30.));
        }
    }
}

fn test_vp9(bgras: &Vec<Vec<u8>>, width: usize, height: usize, bitrate_k: usize, count: usize) {
    let config = EncoderCfg::VPX(VpxEncoderConfig {
        width: width as _,
        height: height as _,
        timebase: [1, 1000],
        bitrate: bitrate_k as _,
        codec: VpxVideoCodecId::VP9,
        num_threads: (num_cpus::get() / 2) as _,
    });
    let mut encoder = VpxEncoder::new(config).unwrap();
    let mut yuvs = vec![];
    for bgra in bgras {
        let mut yuv = vec![];
        bgra_to_i420(width, height, &bgra, &mut yuv);
        yuvs.push(yuv);
    }
    let start = Instant::now();
    for yuv in yuvs.iter() {
        let _ = encoder
            .encode(start.elapsed().as_millis() as _, yuv, STRIDE_ALIGN)
            .unwrap();
        let _ = encoder.flush().unwrap();
    }
    println!("vp9 encode: {:?}", start.elapsed() / count as u32);

    // prepare data separately
    let mut vp9s = vec![];
    let start = Instant::now();
    for yuv in yuvs {
        for ref frame in encoder
            .encode(start.elapsed().as_millis() as _, &yuv, STRIDE_ALIGN)
            .unwrap()
        {
            vp9s.push(frame.data.to_vec());
        }
        for ref frame in encoder.flush().unwrap() {
            vp9s.push(frame.data.to_vec());
        }
    }
    assert_eq!(vp9s.len(), count);

    let mut decoder = VpxDecoder::new(VpxDecoderConfig {
        codec: VpxVideoCodecId::VP9,
        num_threads: (num_cpus::get() / 2) as _,
    })
    .unwrap();
    let start = Instant::now();
    for vp9 in vp9s {
        let _ = decoder.decode(&vp9);
        let _ = decoder.flush();
    }
    println!("vp9 decode: {:?}", start.elapsed() / count as u32);
}

#[cfg(feature = "hwcodec")]
mod hw {
    use super::*;
    use hwcodec::{
        decode::{DecodeContext, Decoder},
        encode::{EncodeContext, Encoder},
        ffmpeg::{ffmpeg_linesize_offset_length, CodecInfo, CodecInfos},
        AVPixelFormat,
        Quality::*,
        RateControl::*,
    };
    use scrap::{
        convert::hw::{hw_bgra_to_i420, hw_bgra_to_nv12},
        HW_STRIDE_ALIGN,
    };

    pub(crate) fn test(
        bgras: Vec<Vec<u8>>,
        width: usize,
        height: usize,
        bitrate_k: usize,
        yuv_count: usize,
        hw_pixfmt: Pixfmt,
    ) {
        let pixfmt = match hw_pixfmt {
            Pixfmt::I420 => AVPixelFormat::AV_PIX_FMT_YUV420P,
            Pixfmt::NV12 => AVPixelFormat::AV_PIX_FMT_NV12,
        };
        let yuvs = hw::brga_to_yuv(bgras, width, height, pixfmt);
        let ctx = EncodeContext {
            name: String::from(""),
            width: width as _,
            height: height as _,
            pixfmt,
            align: 0,
            bitrate: (bitrate_k * 1000) as _,
            timebase: [1, 30],
            gop: 60,
            quality: Quality_Default,
            rc: RC_DEFAULT,
        };

        let encoders = Encoder::available_encoders(ctx.clone());
        println!("hw encoders: {}", encoders.len());
        let best = CodecInfo::score(encoders.clone());
        for info in encoders {
            test_encoder(info.clone(), ctx.clone(), &yuvs, is_best(&best, &info));
        }

        let (h264s, h265s) = prepare_h26x(best, ctx.clone(), &yuvs);
        assert!(h264s.is_empty() || h264s.len() == yuv_count);
        assert!(h265s.is_empty() || h265s.len() == yuv_count);
        let decoders = Decoder::available_decoders();
        println!("hw decoders: {}", decoders.len());
        let best = CodecInfo::score(decoders.clone());
        for info in decoders {
            let h26xs = if info.name.contains("h264") {
                &h264s
            } else {
                &h265s
            };
            if h26xs.len() == yuvs.len() {
                test_decoder(info.clone(), h26xs, is_best(&best, &info));
            }
        }
    }

    fn test_encoder(info: CodecInfo, ctx: EncodeContext, yuvs: &Vec<Vec<u8>>, best: bool) {
        let mut ctx = ctx;
        ctx.name = info.name;
        let mut encoder = Encoder::new(ctx.clone()).unwrap();
        let start = Instant::now();
        for yuv in yuvs {
            let _ = encoder.encode(yuv).unwrap();
        }
        println!(
            "{}{}: {:?}",
            if best { "*" } else { "" },
            ctx.name,
            start.elapsed() / yuvs.len() as u32
        );
    }

    fn test_decoder(info: CodecInfo, h26xs: &Vec<Vec<u8>>, best: bool) {
        let ctx = DecodeContext {
            name: info.name,
            device_type: info.hwdevice,
        };

        let mut decoder = Decoder::new(ctx.clone()).unwrap();
        let start = Instant::now();
        let mut cnt = 0;
        for h26x in h26xs {
            let _ = decoder.decode(h26x).unwrap();
            cnt += 1;
        }
        let device = format!("{:?}", ctx.device_type).to_lowercase();
        let device = device.split("_").last().unwrap();
        println!(
            "{}{} {}: {:?}",
            if best { "*" } else { "" },
            ctx.name,
            device,
            start.elapsed() / cnt
        );
    }

    fn prepare_h26x(
        best: CodecInfos,
        ctx: EncodeContext,
        yuvs: &Vec<Vec<u8>>,
    ) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
        let f = |info: Option<CodecInfo>| {
            let mut h26xs = vec![];
            if let Some(info) = info {
                let mut ctx = ctx.clone();
                ctx.name = info.name;
                let mut encoder = Encoder::new(ctx).unwrap();
                for yuv in yuvs {
                    let h26x = encoder.encode(yuv).unwrap();
                    for frame in h26x {
                        h26xs.push(frame.data.to_vec());
                    }
                }
            }
            h26xs
        };
        (f(best.h264), f(best.h265))
    }

    fn is_best(best: &CodecInfos, info: &CodecInfo) -> bool {
        Some(info.clone()) == best.h264 || Some(info.clone()) == best.h265
    }

    pub fn brga_to_yuv(
        bgras: Vec<Vec<u8>>,
        width: usize,
        height: usize,
        pixfmt: AVPixelFormat,
    ) -> Vec<Vec<u8>> {
        let mut v = vec![];
        let (linesize, offset, length) =
            ffmpeg_linesize_offset_length(pixfmt, width, height, HW_STRIDE_ALIGN).unwrap();
        for bgra in bgras {
            let mut yuv = vec![];
            if pixfmt == AVPixelFormat::AV_PIX_FMT_YUV420P {
                hw_bgra_to_i420(width, height, &linesize, &offset, length, &bgra, &mut yuv);
            } else {
                hw_bgra_to_nv12(width, height, &linesize, &offset, length, &bgra, &mut yuv);
            }
            v.push(yuv);
        }
        v
    }
}
