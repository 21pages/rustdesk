use docopt::Docopt;
use hbb_common::env_logger::{init_from_env, Env, DEFAULT_FILTER_ENV};
use scrap::{
    codec::{EncoderApi, EncoderCfg},
    Capturer, Display, TraitCapturer, VpxDecoder, VpxDecoderConfig, VpxEncoder, VpxEncoderConfig,
    VpxVideoCodecId::{self, *},
    STRIDE_ALIGN,
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
  --hw-pixfmt=PIXFMT    Hardware codec pixfmt. [default: nv12]
                        Valid values: i420, nv12.
";

#[derive(Debug, serde::Deserialize)]
struct Args {
    flag_count: usize,
    flag_bitrate: usize,
    flag_hw_pixfmt: Pixfmt,
}

#[derive(Debug, serde::Deserialize)]
enum Pixfmt {
    I420,
    NV12,
}

fn main() {
    init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let bitrate_k = args.flag_bitrate;
    let yuv_count = args.flag_count;
    let (yuvs, width, height) = capture_yuv(yuv_count);
    println!(
        "benchmark {}x{} bitrate:{}k hw_pixfmt:{:?}",
        width, height, bitrate_k, args.flag_hw_pixfmt
    );
    [VP8, VP9].map(|c| {
        test_vpx(
            c,
            &yuvs,
            width as usize,
            height as usize,
            bitrate_k,
            yuv_count,
        )
    });
    #[cfg(feature = "hwcodec")]
    {
        use hw_common::PixelFormat;
        let hw_pixfmt = match args.flag_hw_pixfmt {
            Pixfmt::I420 => PixelFormat::I420,
            Pixfmt::NV12 => PixelFormat::NV12,
        };
        let yuvs = hw::vpx_yuv_to_hw_yuv(yuvs, width, height, hw_pixfmt);
        hw::test(&yuvs, width, height, bitrate_k, yuv_count, hw_pixfmt);
    }
}

fn capture_yuv(yuv_count: usize) -> (Vec<Vec<u8>>, i32, i32) {
    let mut index = 0;
    let mut displays = Display::all().unwrap();
    for i in 0..displays.len() {
        if displays[i].is_primary() {
            index = i;
            break;
        }
    }
    let d = displays.remove(index);
    let mut c = Capturer::new(d, true).unwrap();
    let mut v = vec![];
    loop {
        if let Ok(frame) = c.frame(std::time::Duration::from_millis(30)) {
            v.push(frame.0.to_vec());
            print!("\rcapture {}/{}", v.len(), yuv_count);
            std::io::stdout().flush().ok();
            if v.len() == yuv_count {
                println!();
                return (v, c.width() as i32, c.height() as i32);
            }
        }
    }
}

fn test_vpx(
    codec_id: VpxVideoCodecId,
    yuvs: &Vec<Vec<u8>>,
    width: usize,
    height: usize,
    bitrate_k: usize,
    yuv_count: usize,
) {
    let config = EncoderCfg::VPX(VpxEncoderConfig {
        width: width as _,
        height: height as _,
        timebase: [1, 1000],
        bitrate: bitrate_k as _,
        codec: codec_id,
        num_threads: (num_cpus::get() / 2) as _,
    });
    let mut encoder = VpxEncoder::new(config).unwrap();
    let start = Instant::now();
    for yuv in yuvs {
        let _ = encoder
            .encode(start.elapsed().as_millis() as _, yuv, STRIDE_ALIGN)
            .unwrap();
        let _ = encoder.flush().unwrap();
    }
    println!(
        "{:?} encode: {:?}",
        codec_id,
        start.elapsed() / yuv_count as _
    );

    // prepare data separately
    let mut vpxs = vec![];
    let start = Instant::now();
    for yuv in yuvs {
        for ref frame in encoder
            .encode(start.elapsed().as_millis() as _, yuv, STRIDE_ALIGN)
            .unwrap()
        {
            vpxs.push(frame.data.to_vec());
        }
        for ref frame in encoder.flush().unwrap() {
            vpxs.push(frame.data.to_vec());
        }
    }
    assert_eq!(vpxs.len(), yuv_count);

    let mut decoder = VpxDecoder::new(VpxDecoderConfig {
        codec: codec_id,
        num_threads: (num_cpus::get() / 2) as _,
    })
    .unwrap();
    let start = Instant::now();
    for vpx in vpxs {
        let _ = decoder.decode(&vpx);
        let _ = decoder.flush();
    }
    println!(
        "{:?} decode: {:?}",
        codec_id,
        start.elapsed() / yuv_count as _
    );
}

#[cfg(feature = "hwcodec")]
mod hw {
    use super::*;
    use hw_common::{DataFormat, DecodeContext, EncodeContext, PixelFormat, PresetContext};
    use hwcodec::{
        decode::{self, Decoder},
        encode::{self, Encoder},
    };
    use scrap::convert::{
        hw::{hw_bgra_to_i420, hw_bgra_to_nv12, linesize_offset_length, split_yuv},
        i420_to_bgra,
    };

    pub fn test(
        yuvs: &Vec<Vec<u8>>,
        width: i32,
        height: i32,
        _bitrate_k: usize,
        yuv_count: usize,
        _pixfmt: PixelFormat,
    ) {
        let mut encoders = encode::available(
            PresetContext {
                width: 1920,
                height: 1080,
            },
            DynamicContext {
                kbitrate: 5000,
                framerate: 60,
            },
        );
        encoders
            .iter_mut()
            .map(|e| {
                e.width = width;
                e.height = height
            })
            .count();
        println!("hw encoders: {}", encoders.len());
        let best = encode::Best::new(encoders.clone());
        for ctx in encoders {
            test_encoder(ctx.clone(), yuvs, is_best_encoder(&best, &ctx));
        }

        let (h264s, h265s) = prepare_h26x(best, yuvs);
        assert!(h264s.is_empty() || h264s.len() == yuv_count);
        assert!(h265s.is_empty() || h265s.len() == yuv_count);
        let decoders = decode::available();
        println!("hw decoders: {}", decoders.len());
        let best = decode::Best::new(decoders.clone());
        for ctx in decoders {
            let h26xs = if ctx.dataFormat == DataFormat::H264 {
                &h264s
            } else {
                &h265s
            };
            if h26xs.len() == yuvs.len() {
                test_decoder(ctx.clone(), h26xs, is_best_decoder(&best, &ctx));
            }
        }
    }

    fn test_encoder(ctx: EncodeContext, yuvs: &Vec<Vec<u8>>, best: bool) {
        let mut encoder = Encoder::new(ctx.clone()).unwrap();
        let start = Instant::now();
        for yuv in yuvs {
            let (datas, linesizes) = split_yuv(ctx.f.pixfmt, ctx.p.width, ctx.p.height, &yuv);
            let _ = encoder.encode(datas, linesizes).unwrap();
        }
        println!(
            "{}{:?} {:?} {:?}: {:?}",
            if best { "*" } else { "" },
            ctx.driver,
            ctx.dataFormat,
            ctx.device,
            start.elapsed() / yuvs.len() as _
        );
    }

    fn test_decoder(ctx: DecodeContext, h26xs: &Vec<Vec<u8>>, best: bool) {
        let mut decoder = Decoder::new(ctx.clone()).unwrap();
        let start = Instant::now();
        let mut cnt = 0;
        for h26x in h26xs {
            let _ = decoder.decode(h26x).unwrap();
            cnt += 1;
        }
        println!(
            "{}{:?} {:?} {:?}: {:?}",
            if best { "*" } else { "" },
            ctx.driver,
            ctx.dataFormat,
            ctx.device,
            start.elapsed() / cnt
        );
    }

    fn prepare_h26x(best: encode::Best, yuvs: &Vec<Vec<u8>>) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
        let f = |ctx: Option<EncodeContext>| {
            let mut h26xs = vec![];
            if let Some(ctx) = ctx {
                let mut encoder = Encoder::new(ctx.clone()).unwrap();
                for yuv in yuvs {
                    let (datas, linesizes) = split_yuv(ctx.pixfmt, ctx.width, ctx.height, &yuv);
                    let h26x = encoder.encode(datas, linesizes).unwrap();
                    for frame in h26x {
                        h26xs.push(frame.data.to_vec());
                    }
                }
            }
            h26xs
        };
        (f(best.h264), f(best.h265))
    }

    fn is_best_encoder(best: &encode::Best, ctx: &EncodeContext) -> bool {
        Some(ctx.clone()) == best.h264 || Some(ctx.clone()) == best.h265
    }

    fn is_best_decoder(best: &decode::Best, ctx: &DecodeContext) -> bool {
        Some(ctx.clone()) == best.h264 || Some(ctx.clone()) == best.h265
    }

    pub fn vpx_yuv_to_hw_yuv(
        yuvs: Vec<Vec<u8>>,
        width: i32,
        height: i32,
        pixfmt: PixelFormat,
    ) -> Vec<Vec<u8>> {
        let yuvs = yuvs;
        let mut bgra = vec![];
        let mut v = vec![];
        let (linesize, offset, length) = linesize_offset_length(pixfmt, width, height);
        for mut yuv in yuvs {
            i420_to_bgra(width as usize, height as usize, &yuv, &mut bgra);
            if pixfmt == PixelFormat::I420 {
                hw_bgra_to_i420(width, height, &linesize, &offset, length, &bgra, &mut yuv);
            } else {
                hw_bgra_to_nv12(width, height, &linesize, &offset, length, &bgra, &mut yuv);
            }
            v.push(yuv);
        }
        v
    }
}
