use docopt::Docopt;
use hbb_common::env_logger::{init_from_env, Env, DEFAULT_FILTER_ENV};
use scrap::{
    codec::{EncoderApi, EncoderCfg},
    VpxDecoder, VpxDecoderConfig, VpxEncoder, VpxEncoderConfig, VpxVideoCodecId, STRIDE_ALIGN,
};
use std::{io::Write, path::PathBuf};

// cargo run --package scrap --example resolution --features hwcodec -- --width=1920 --height=1080

const USAGE: &'static str = "
Resolution test.

Usage:
    resolution [--count=COUNT] [--width=WIDTH] [--height=HEIGHT]
    resolution (-h | --help)

Options:
    -h --help             Show this screen.
    --width=WIDTH         Video width. [default: 1920]
    --height=HEIGHT       Video height. [default: 1080]
";

#[derive(Debug, serde::Deserialize)]
struct Args {
    flag_width: usize,
    flag_height: usize,
}

fn main() {
    init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let width = args.flag_width;
    let height = args.flag_height;
    println!("resolution: {}x{}", width, height);
    let dir = PathBuf::from(format!("./output/{}x{}", width, height));
    std::fs::create_dir_all(&dir).unwrap();
    let bgra = generate_bgra(width, height);
    test_vp9(&bgra, width, height, &dir);
    #[cfg(feature = "hwcodec")]
    hw::test(
        &bgra,
        width as i32,
        height as i32,
        hw_common::PixelFormat::NV12,
        &dir,
    );
}

fn generate_bgra(width: usize, height: usize) -> Vec<u8> {
    let mut v = vec![0u8; width * height * 4];
    for h in 0..height {
        for w in 0..width {
            let offset = (width * h + w) * 4;
            v[offset] = w as u8;
            v[offset + 1] = h as u8;
            v[offset + 2] = (w + h) as u8;
            v[offset + 3] = 255;
        }
    }
    v
}

fn test_vp9(bgra: &Vec<u8>, width: usize, height: usize, dir: &PathBuf) {
    print!("VP9: ");
    let config = EncoderCfg::VPX(VpxEncoderConfig {
        width: width as _,
        height: height as _,
        timebase: [1, 1000],
        bitrate: 5000,
        codec: VpxVideoCodecId::VP9,
        num_threads: (num_cpus::get() / 2) as _,
    });
    let mut encoder = VpxEncoder::new(config).unwrap();
    let mut yuv = vec![];
    scrap::convert::bgra_to_i420(width, height, bgra, &mut yuv);
    let mut vp9s = vec![];
    for frame in encoder.encode(0, &yuv, STRIDE_ALIGN).unwrap() {
        vp9s.push(frame.data.to_vec());
    }
    for frame in encoder.flush().unwrap() {
        vp9s.push(frame.data.to_vec());
    }
    let mut file = std::fs::File::create(dir.join("vp9.vp9")).unwrap();
    let mut decoder = VpxDecoder::new(VpxDecoderConfig {
        codec: VpxVideoCodecId::VP9,
        num_threads: (num_cpus::get() / 2) as _,
    })
    .unwrap();
    for vp9 in vp9s {
        file.write_all(&vp9).unwrap();
        let mut img = scrap::vpxcodec::Image::new();
        for frame in decoder.decode(&vp9).unwrap() {
            drop(img);
            img = frame;
        }
        for frame in decoder.flush().unwrap() {
            drop(img);
            img = frame;
        }
        println!("w:{}, h:{}", img.width(), img.height());
    }
}

#[cfg(feature = "hwcodec")]
mod hw {
    use super::*;
    use hw_common::{DataFormat, DecodeContext, EncodeContext, PixelFormat};
    use hwcodec::{
        decode::{self, Decoder},
        encode::{self, Encoder},
    };
    use scrap::convert::hw::{hw_bgra_to_nv12, linesize_offset_length, split_yuv};

    pub fn test(bgra: &Vec<u8>, width: i32, height: i32, pixfmt: PixelFormat, dir: &PathBuf) {
        let (linesize, offset, len) = linesize_offset_length(pixfmt, width as i32, height as i32);
        let mut yuv = vec![];
        hw_bgra_to_nv12(
            width as i32,
            height as i32,
            &linesize,
            &offset,
            len,
            &bgra,
            &mut yuv,
        );
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
            test_encoder(ctx.clone(), &yuv, dir);
        }

        let (h264, h265) = prepare_h26x(best, &yuv);
        let decoders = decode::available();
        println!("hw decoders: {}", decoders.len());
        for ctx in decoders {
            let h26x = if ctx.dataFormat == DataFormat::H264 {
                &h264
            } else {
                &h265
            };
            if h26x.len() > 0 {
                test_decoder(ctx.clone(), h26x);
            }
        }
    }

    fn test_encoder(ctx: EncodeContext, yuv: &Vec<u8>, dir: &PathBuf) {
        println!("{:?} {:?} {:?} ", ctx.driver, ctx.dataFormat, ctx.device);
        let mut encoder = Encoder::new(ctx.clone()).unwrap();
        let (datas, linesizes) = split_yuv(ctx.pixfmt, ctx.width, ctx.height, yuv);
        let frames = encoder.encode(datas, linesizes).unwrap();
        let filename = format!(
            "{:?}_{:?}.{}",
            ctx.driver,
            ctx.device,
            if ctx.dataFormat == DataFormat::H264 {
                "264"
            } else {
                "265"
            }
        );
        let mut file = std::fs::File::create(dir.join(filename)).unwrap();
        file.write_all(&frames[0].data).unwrap();
    }

    fn test_decoder(ctx: DecodeContext, h26x: &Vec<u8>) {
        print!("{:?} {:?} {:?}: ", ctx.driver, ctx.dataFormat, ctx.device,);
        let mut decoder = Decoder::new(ctx.clone()).unwrap();
        for frame in decoder.decode(h26x).unwrap() {
            println!("w:{}, h:{}", frame.width, frame.height);
        }
    }

    fn prepare_h26x(best: encode::Best, yuv: &Vec<u8>) -> (Vec<u8>, Vec<u8>) {
        let f = |ctx: Option<EncodeContext>| {
            if let Some(ctx) = ctx {
                let mut encoder = Encoder::new(ctx.clone()).unwrap();
                let (datas, linesizes) = split_yuv(ctx.pixfmt, ctx.width, ctx.height, &yuv);
                let h26x = encoder.encode(datas, linesizes).unwrap();
                return h26x[0].data.to_vec();
            }
            vec![]
        };
        (f(best.h264), f(best.h265))
    }
}
