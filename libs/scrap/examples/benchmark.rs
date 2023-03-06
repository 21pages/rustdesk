use docopt::Docopt;
use hbb_common::env_logger::{init_from_env, Env, DEFAULT_FILTER_ENV};
use scrap::{
    bgra_to_i420,
    codec::{EncoderApi, EncoderCfg},
    Capturer, Display, TraitCapturer, VpxDecoder, VpxDecoderConfig, VpxEncoder, VpxEncoderConfig,
    VpxVideoCodecId, STRIDE_ALIGN,
};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

// cargo run --package scrap --example benchmark --release --features hwcodec -- --dir="D:/tmp"

const USAGE: &'static str = "
Codec benchmark.

Usage:
  benchmark [--dir=DIR] [--count=COUNT] [--bitrate=KBS] [--hw-pixfmt=PIXFMT]
  benchmark (-h | --help)

Options:
  -h --help             Show this screen.
  --dir=DIR             Video file save directory.
  --count=COUNT         Capture frame count [default: 100].
  --bitrate=KBS         Video bitrate in kilobits per second [default: 5000].
  --hw-pixfmt=PIXFMT    Hardware codec pixfmt. [default: i420]
                        Valid values: i420, nv12.
";

#[derive(Debug, serde::Deserialize)]
struct Args {
    flag_dir: String,
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
    let dir = PathBuf::from(&args.flag_dir);
    if !dir.exists() {
        println!("{} not exist", args.flag_dir);
        return;
    }
    let (width, height) = capture_bgra(dir.clone(), count);
    println!(
        "benchmark {}x{} bitrate:{}k hw_pixfmt:{:?}",
        width, height, bitrate_k, args.flag_hw_pixfmt
    );
    test_vp9(dir.clone(), width, height, bitrate_k, count);
    #[cfg(feature = "hwcodec")]
    hw::test(
        dir.clone(),
        width,
        height,
        bitrate_k,
        count,
        args.flag_hw_pixfmt,
    );
}

fn capture_bgra(dir: PathBuf, count: usize) -> (usize, usize) {
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
    let mut file = File::create(dir.join("bm.bgra")).unwrap();
    let mut counter = 0;
    loop {
        if let Ok(frame) = c.frame(std::time::Duration::from_secs_f32(1. / 30.)) {
            counter += 1;
            file.write(frame.0).unwrap();
            file.flush().unwrap();
            print!("\rcapture {}/{}", counter, count);
            std::io::stdout().flush().ok();
            if counter == count {
                println!();
                return (c.width(), c.height());
            }
            std::thread::sleep(std::time::Duration::from_secs_f32(1. / 30.));
        }
    }
}

fn test_vp9(dir: PathBuf, width: usize, height: usize, bitrate_k: usize, count: usize) {
    let config = EncoderCfg::VPX(VpxEncoderConfig {
        width: width as _,
        height: height as _,
        timebase: [1, 1000],
        bitrate: bitrate_k as _,
        codec: VpxVideoCodecId::VP9,
        num_threads: (num_cpus::get() / 2) as _,
    });
    let mut encoder = VpxEncoder::new(config).unwrap();
    let mut bgra = vec![0u8; width * height * 4];
    let mut yuv = vec![];
    let mut file = File::open(dir.join("bm.bgra")).unwrap();
    let mut sum = Duration::ZERO;
    let mut vp9s = vec![];
    for _ in 0..count {
        assert_eq!(file.read(&mut bgra).unwrap(), bgra.len());
        bgra_to_i420(width, height, &bgra, &mut yuv);
        let start = Instant::now();
        let frame1 = encoder
            .encode(start.elapsed().as_millis() as _, &yuv, STRIDE_ALIGN)
            .unwrap();
        for ref frame in frame1 {
            vp9s.push(frame.data.to_vec());
        }
        let frame2 = encoder.flush().unwrap();
        sum += start.elapsed();
        for ref frame in frame2 {
            vp9s.push(frame.data.to_vec());
        }
    }
    println!("vp9 encode: {:?}", sum / count as u32);

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
        dir: PathBuf,
        width: usize,
        height: usize,
        bitrate_k: usize,
        count: usize,
        hw_pixfmt: Pixfmt,
    ) {
        let pixfmt = match hw_pixfmt {
            Pixfmt::I420 => AVPixelFormat::AV_PIX_FMT_YUV420P,
            Pixfmt::NV12 => AVPixelFormat::AV_PIX_FMT_NV12,
        };
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
        let mut h264s = vec![];
        let mut h265s = vec![];
        let (linesize, offset, length) =
            ffmpeg_linesize_offset_length(pixfmt, width, height, HW_STRIDE_ALIGN).unwrap();
        let mut bgra = vec![0u8; width * height * 4];
        let mut yuv = vec![];
        for info in encoders {
            let mut sum = Duration::ZERO;
            let mut ctx = ctx.clone();
            ctx.name = info.name.clone();
            let is264 = ctx.name.contains("264");
            let is_best = is_best(&best, &info);
            let mut encoder = Encoder::new(ctx.clone()).unwrap();
            let mut file = File::open(dir.join("bm.bgra")).unwrap();
            for _ in 0..count {
                assert_eq!(file.read(&mut bgra).unwrap(), bgra.len());
                if pixfmt == AVPixelFormat::AV_PIX_FMT_YUV420P {
                    hw_bgra_to_i420(width, height, &linesize, &offset, length, &bgra, &mut yuv);
                } else {
                    hw_bgra_to_nv12(width, height, &linesize, &offset, length, &bgra, &mut yuv);
                }
                let start = Instant::now();
                let frame = encoder.encode(&yuv).unwrap();
                sum += start.elapsed();
                if is_best {
                    for ref frame in frame {
                        if is264 {
                            h264s.push(frame.data.to_vec());
                        } else {
                            h265s.push(frame.data.to_vec());
                        }
                    }
                }
            }
            println!(
                "{}{}: {:?}",
                if is_best { "*" } else { "" },
                ctx.name,
                sum / count as u32
            );
        }

        assert!(h264s.is_empty() || h264s.len() == count);
        assert!(h265s.is_empty() || h265s.len() == count);
        let decoders = Decoder::available_decoders();
        println!("hw decoders: {}", decoders.len());
        let best = CodecInfo::score(decoders.clone());
        for info in decoders {
            let h26xs = if info.name.contains("h264") {
                &h264s
            } else {
                &h265s
            };
            if h26xs.len() == count {
                test_decoder(info.clone(), h26xs, is_best(&best, &info));
            }
        }
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
    fn is_best(best: &CodecInfos, info: &CodecInfo) -> bool {
        Some(info.clone()) == best.h264 || Some(info.clone()) == best.h265
    }
}
