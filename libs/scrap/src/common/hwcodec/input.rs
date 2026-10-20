use crate::{convert::*, Pixfmt, TraitPixelBuffer};
use hbb_common::{anyhow::anyhow, bail, log, ResultType};
use hwcodec::{ffmpeg::AVPixelFormat, ffmpeg_ram::encode::WritableFrame};

crate::generate_call_macro!(call_yuv, false);

pub(super) fn convert(
    captured: &dyn TraitPixelBuffer,
    frame: &mut WritableFrame<'_>,
    scratch: &mut Vec<u8>,
) -> ResultType<()> {
    let width = captured.width();
    let height = captured.height();
    let format = captured.pixfmt();
    if width == 0 || height == 0 || width > frame.width() || height > frame.height() {
        bail!("Invalid RAM input dimensions: {width}x{height}");
    }
    if !matches!(format, Pixfmt::BGRA | Pixfmt::RGBA | Pixfmt::RGB565LE) {
        bail!("Unsupported RAM input format: {format:?}");
    }
    let stride = captured
        .stride()
        .first()
        .copied()
        .ok_or_else(|| anyhow!("Missing RAM input stride"))?;
    let row_bytes = width
        .checked_mul(format.bytes_per_pixel())
        .ok_or_else(|| anyhow!("RAM input row overflow"))?;
    let source_len = stride
        .checked_mul(height)
        .ok_or_else(|| anyhow!("RAM input size overflow"))?;
    let source = captured.data();
    if stride < row_bytes || stride > i32::MAX as usize || source.len() < source_len {
        bail!("Invalid RAM input stride or length");
    }

    let output_format = frame.pixfmt();
    let strides = [
        frame.linesize()[0],
        frame.linesize()[1],
        frame.linesize()[2],
    ];
    let padded = width != frame.width() || height != frame.height();
    let [y, u, v] = frame.planes_mut();
    // Odd capture sizes are rounded up by the encoder. Initialize the uncovered
    // edge so replacing a retained buffer never depends on the previous image.
    if padded {
        y.fill(0);
        u.fill(128);
        v.fill(128);
    }
    match output_format {
        AVPixelFormat::AV_PIX_FMT_NV12 => {
            let (input, input_stride) = if format == Pixfmt::RGB565LE {
                let argb_stride = width
                    .checked_mul(4)
                    .filter(|&stride| stride <= i32::MAX as usize)
                    .ok_or_else(|| anyhow!("RGB intermediate stride overflow"))?;
                let argb_len = argb_stride
                    .checked_mul(height)
                    .ok_or_else(|| anyhow!("RGB intermediate size overflow"))?;
                scratch.resize(argb_len, 0);
                call_yuv!(RGB565ToARGB(
                    source.as_ptr(),
                    stride as _,
                    scratch.as_mut_ptr(),
                    argb_stride as _,
                    width as _,
                    height as _,
                ));
                (scratch.as_ptr(), argb_stride)
            } else {
                (source.as_ptr(), stride)
            };
            let convert = if format == Pixfmt::RGBA {
                ABGRToNV12
            } else {
                ARGBToNV12
            };
            call_yuv!(convert(
                input,
                input_stride as _,
                y.as_mut_ptr(),
                strides[0],
                u.as_mut_ptr(),
                strides[1],
                width as _,
                height as _,
            ));
        }
        AVPixelFormat::AV_PIX_FMT_YUV420P => {
            let convert = match format {
                Pixfmt::BGRA => ARGBToI420,
                Pixfmt::RGBA => ABGRToI420,
                Pixfmt::RGB565LE => RGB565ToI420,
                _ => bail!("Unsupported RAM input format"),
            };
            call_yuv!(convert(
                source.as_ptr(),
                stride as _,
                y.as_mut_ptr(),
                strides[0],
                u.as_mut_ptr(),
                strides[1],
                v.as_mut_ptr(),
                strides[2],
                width as _,
                height as _,
            ));
        }
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::{codec::EncoderApi, EncodeInput, EncodeYuvFormat, PixelBuffer};
    use hwcodec::{
        common::{Quality, RateControl},
        ffmpeg_ram::encode::{EncodeContext, Encoder},
    };

    #[test]
    #[ignore = "requires Intel QSV hardware"]
    fn direct_conversion_matches_existing_yuv_conversion() {
        let mut cases = 0;
        for format in [Pixfmt::BGRA, Pixfmt::RGBA, Pixfmt::RGB565LE] {
            for (width, height) in [(128, 96), (127, 95)] {
                for align in [0, 256] {
                    let stride = width * format.bytes_per_pixel() + 16;
                    let bytes: Vec<u8> = (0..stride * height)
                        .map(|i| ((i * 13 + i / stride * 7) % 251) as u8)
                        .collect();
                    let pixels = PixelBuffer::new(&bytes, format, width, height);
                    let mut encoder = Encoder::new(EncodeContext {
                        name: "h264_qsv".into(),
                        mc_name: None,
                        width: 128,
                        height: 96,
                        pixfmt: AVPixelFormat::AV_PIX_FMT_NV12,
                        align,
                        fps: 30,
                        gop: 60,
                        rc: RateControl::RC_CBR,
                        quality: Quality::Quality_Default,
                        kbs: 1000,
                        q: -1,
                        thread_count: 1,
                    })
                    .unwrap();
                    let reference_format = EncodeYuvFormat {
                        pixfmt: Pixfmt::NV12,
                        w: 128,
                        h: 96,
                        stride: encoder.linesize.iter().map(|&s| s as usize).collect(),
                        u: encoder.offset[0] as usize,
                        v: 0,
                    };
                    let mut reference = Vec::new();
                    crate::convert_to_yuv(
                        &pixels,
                        reference_format.clone(),
                        &mut reference,
                        &mut Vec::new(),
                    )
                    .unwrap();
                    let mut frame = encoder.prepare_frame().unwrap();
                    convert(&pixels, &mut frame, &mut Vec::new()).unwrap();
                    let strides = [frame.linesize()[0] as usize, frame.linesize()[1] as usize];
                    let [y, uv, _] = frame.planes_mut();
                    for (i, (plane, offset, rows, columns)) in [
                        (&*y, 0, height, width),
                        (
                            &*uv,
                            reference_format.u,
                            (height + 1) / 2,
                            (width + 1) / 2 * 2,
                        ),
                    ]
                    .iter()
                    .copied()
                    .enumerate()
                    {
                        for row in 0..rows {
                            let actual = &plane[row * strides[i]..][..columns];
                            let expected =
                                &reference[offset + row * reference_format.stride[i]..][..columns];
                            assert_eq!(
                                actual, expected,
                                "{format:?} {width}x{height} align={align}"
                            );
                        }
                    }
                    assert!(!frame.encode(0).unwrap().is_empty());
                    assert!(!encoder.reencode(33).unwrap().is_empty());
                    let mut abandoned = encoder.prepare_frame().unwrap();
                    let invalid = PixelBuffer::new(&bytes[..1], format, width, height);
                    assert!(convert(&invalid, &mut abandoned, &mut Vec::new()).is_err());
                    drop(abandoned);
                    assert!(encoder.reencode(66).is_err());
                    cases += 1;
                }
            }
        }
        println!("PASS {cases} direct conversion/reference/repeat cases");
    }

    #[test]
    #[ignore = "requires Intel QSV hardware"]
    fn pixelbuffer_and_repeat_messages() {
        use crate::{
            codec::EncoderCfg,
            hwcodec::{HwRamEncoder, HwRamEncoderConfig},
        };
        let mut encoder = HwRamEncoder::new(
            EncoderCfg::HWRAM(HwRamEncoderConfig {
                name: "h264_qsv".into(),
                mc_name: None,
                width: 128,
                height: 96,
                quality: 1.0,
                keyframe_interval: Some(60),
            }),
            false,
        )
        .unwrap();
        assert!(encoder.input_pixelbuffer());
        assert!(encoder.encode_to_message(EncodeInput::Repeat, 0).is_err());
        let mut bytes = vec![90; 128 * 96 * 4];
        let first = encoder
            .encode_to_message(
                EncodeInput::PixelBuffer(&PixelBuffer::with_BGRA(&bytes, 128, 96)),
                0,
            )
            .unwrap();
        bytes.fill(0);
        drop(bytes);
        let repeated = encoder.encode_to_message(EncodeInput::Repeat, 33).unwrap();
        assert!(first.has_h264s() && repeated.has_h264s());
        assert_eq!(repeated.h264s().frames[0].pts, 33);
    }
}
