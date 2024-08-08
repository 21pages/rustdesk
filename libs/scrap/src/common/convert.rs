#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(improper_ctypes)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/yuv_ffi.rs"));

#[cfg(not(target_os = "ios"))]
use crate::PixelBuffer;
use crate::{generate_call_macro, EncodeYuvFormat, TraitPixelBuffer};
use hbb_common::{bail, log, ResultType};

generate_call_macro!(call_yuv, false);

/*
https://github.com/newOcean/webrtc-ios/blob/44e5f2bc1bbb6ab6372fd14308f355f5e0fb70d5/trunk/talk/media/base/videoframe_unittest.h#L361
BGR24 -> RAW, RGB24 -> RGB24
*/

#[cfg(not(target_os = "ios"))]
pub fn convert_to_yuv(
    captured: &PixelBuffer,
    dst_fmt: EncodeYuvFormat,
    dst: &mut Vec<u8>,
    mid_data: &mut Vec<u8>,
) -> ResultType<()> {
    use crate::Pixfmt::*;

    // let src = captured.data();
    // let src_pixfmt = captured.pixfmt();
    // let src_stride = captured.stride();
    let src_pixfmt = RGBA;
    let (src, src_stride) = unsafe { test::change_src_fmt_to(captured, src_pixfmt) };
    let src_width = captured.width();
    let src_height = captured.height();
    if src_width > dst_fmt.w || src_height > dst_fmt.h {
        bail!(
            "src rect > dst rect: ({src_width}, {src_height}) > ({},{})",
            dst_fmt.w,
            dst_fmt.h
        );
    }
    // stride is calculated, not real, so we need to check it
    if src_stride[0] < src_width * src_pixfmt.bytes_per_pixel() {
        bail!(
            "src_stride too small: {} < {}",
            src_stride[0],
            src_width * src_pixfmt.bytes_per_pixel()
        );
    }
    if src.len() < src_stride[0] * src_height {
        bail!(
            "wrong src len, {} < {} * {}",
            src.len(),
            src_stride[0],
            src_height
        );
    }
    let align = |x: usize| (x + 63) / 64 * 64;
    let unsupported = format!(
        "unsupported pixfmt conversion: {src_pixfmt:?} -> {:?}",
        dst_fmt.pixfmt
    );

    let mut to_bgra = || {
        let mid_f = match src_pixfmt {
            RGBA => ABGRToARGB,
            ARGB => BGRAToARGB,
            BGR24 => RAWToARGB,
            RGB24 => RGB24ToARGB,
            RGB565LE => RGB565ToARGB,
            RGB555LE => ARGB1555ToARGB,
            _ => bail!(unsupported.clone()),
        };
        let mid_stride = src_width * 4;
        mid_data.resize(mid_stride * src_height, 0);
        call_yuv!(mid_f(
            src.as_ptr(),
            src_stride[0] as _,
            mid_data.as_mut_ptr(),
            mid_stride as _,
            src_width as _,
            src_height as _,
        ));
        Ok(())
    };

    match dst_fmt.pixfmt {
        I420 => {
            let dst_stride_y = dst_fmt.stride[0];
            let dst_stride_uv = dst_fmt.stride[1];
            dst.resize(dst_fmt.h * dst_stride_y * 2, 0); // waste some memory to ensure memory safety
            let dst_y = dst.as_mut_ptr();
            let dst_u = dst[dst_fmt.u..].as_mut_ptr();
            let dst_v = dst[dst_fmt.v..].as_mut_ptr();
            let f = match src_pixfmt {
                BGRA => ARGBToI420,
                ARGB => BGRAToI420,
                RGBA => ABGRToI420,
                BGR24 => RAWToI420,
                RGB24 => RGB24ToI420,
                RGB565LE => RGB565ToI420,
                RGB555LE => ARGB1555ToI420,
                _ => bail!(unsupported),
            };
            call_yuv!(f(
                src.as_ptr(),
                src_stride[0] as _,
                dst_y,
                dst_stride_y as _,
                dst_u,
                dst_stride_uv as _,
                dst_v,
                dst_stride_uv as _,
                src_width as _,
                src_height as _,
            ));
        }
        NV12 => {
            let dst_stride_y = dst_fmt.stride[0];
            let dst_stride_uv = dst_fmt.stride[1];
            dst.resize(
                align(dst_fmt.h) * (align(dst_stride_y) + align(dst_stride_uv / 2)),
                0,
            );
            let dst_y = dst.as_mut_ptr();
            let dst_uv = dst[dst_fmt.u..].as_mut_ptr();
            let (input, input_stride, mid_pixfmt) = match src_pixfmt {
                BGRA => (src.as_ptr(), src_stride[0], src_pixfmt),
                RGBA => (src.as_ptr(), src_stride[0], src_pixfmt),
                _ => {
                    to_bgra()?;
                    (mid_data.as_ptr(), src_width * 4, BGRA)
                }
            };
            let f = match mid_pixfmt {
                BGRA => ARGBToNV12,
                RGBA => ABGRToNV12,
                _ => bail!(unsupported),
            };
            call_yuv!(f(
                input,
                input_stride as _,
                dst_y,
                dst_stride_y as _,
                dst_uv,
                dst_stride_uv as _,
                src_width as _,
                src_height as _,
            ));
        }
        I444 => {
            let dst_stride_y = dst_fmt.stride[0];
            let dst_stride_u = dst_fmt.stride[1];
            let dst_stride_v = dst_fmt.stride[2];
            dst.resize(
                align(dst_fmt.h)
                    * (align(dst_stride_y) + align(dst_stride_u) + align(dst_stride_v)),
                0,
            );
            let dst_y = dst.as_mut_ptr();
            let dst_u = dst[dst_fmt.u..].as_mut_ptr();
            let dst_v = dst[dst_fmt.v..].as_mut_ptr();
            let (input, input_stride) = match src_pixfmt {
                BGRA => (src.as_ptr(), src_stride[0]),
                _ => {
                    to_bgra()?;
                    (mid_data.as_ptr(), src_width * 4)
                }
            };

            call_yuv!(ARGBToI444(
                input,
                input_stride as _,
                dst_y,
                dst_stride_y as _,
                dst_u,
                dst_stride_u as _,
                dst_v,
                dst_stride_v as _,
                src_width as _,
                src_height as _,
            ));
        }
        _ => {
            bail!(unsupported);
        }
    }
    Ok(())
}

#[cfg(not(target_os = "ios"))]
mod test {
    use super::*;
    use crate::Pixfmt;
    use hbb_common::libc::c_int;

    pub(super) unsafe fn change_src_fmt_to<'a>(
        captured: &PixelBuffer,
        dst_fmt: Pixfmt,
    ) -> (Vec<u8>, Vec<usize>) {
        use crate::Pixfmt::*;
        let mut dst = vec![];
        if captured.pixfmt() != BGRA {
            return (dst, vec![]);
        }
        let src = captured.data().as_ptr();
        // let mut src = captured.data().to_vec();
        let src_stride = captured.stride()[0] as c_int;
        let width = captured.width() as c_int;
        let height = captured.height() as c_int;
        let color_height = 32;

        // for h in 0..color_height {
        //     for w in 0..width {
        //         let offset = (src_stride * h + w * 4) as usize;
        //         src[offset + 0] = 0;
        //         src[offset + 1] = 0;
        //         src[offset + 2] = 0xff;
        //     }
        // }

        match dst_fmt {
            ARGB => {
                let dst_stride = width * 4;
                dst.resize((dst_stride * height) as usize, 0);
                ARGBToBGRA(src, src_stride, dst.as_mut_ptr(), dst_stride, width, height);
                for h in 0..color_height {
                    for w in 0..width {
                        let offset = (dst_stride * h + w * 4) as usize;
                        dst[offset + 1] = 0xff;
                        dst[offset + 2] = 0;
                        dst[offset + 3] = 0;
                    }
                }
            }
            RGBA => {
                let dst_stride = width * 4;
                dst.resize((dst_stride * height) as usize, 0);
                ARGBToABGR(src, src_stride, dst.as_mut_ptr(), dst_stride, width, height);
                for h in 0..color_height {
                    for w in 0..width {
                        let offset = (dst_stride * h + w * 4) as usize;
                        dst[offset + 0] = 0xff;
                        dst[offset + 1] = 0;
                        dst[offset + 2] = 0;
                    }
                }
            }
            BGR24 => {
                let dst_stride = width * 3;
                dst.resize((dst_stride * height) as usize, 0);
                ARGBToRAW(src, src_stride, dst.as_mut_ptr(), dst_stride, width, height);
                for h in 0..color_height {
                    for w in 0..width {
                        let offset = (dst_stride * h + w * 3) as usize;
                        dst[offset + 0] = 0;
                        dst[offset + 1] = 0;
                        dst[offset + 2] = 0xff;
                    }
                }
            }
            RGB24 => {
                let dst_stride = width * 3;
                dst.resize((dst_stride * height) as usize, 0);
                ARGBToRGB24(src, src_stride, dst.as_mut_ptr(), dst_stride, width, height);
                for h in 0..color_height {
                    for w in 0..width {
                        let offset = (dst_stride * h + w * 3) as usize;
                        dst[offset + 0] = 0xff;
                        dst[offset + 1] = 0;
                        dst[offset + 2] = 0;
                    }
                }
            }
            RGB565LE => {
                let dst_stride = width * 2;
                dst.resize((dst_stride * height) as usize, 0);
                ARGBToRGB565(src, src_stride, dst.as_mut_ptr(), dst_stride, width, height);
                for h in 0..color_height {
                    for w in 0..width {
                        let offset = (dst_stride * h + w * 2) as usize;
                        // order: BRG
                        dst[offset + 0] = 0x03;
                        dst[offset + 1] = 0xe0;
                    }
                }
            }
            RGB555LE => {
                let dst_stride = width * 2;
                dst.resize((dst_stride * height) as usize, 0);
                ARGBToARGB1555(src, src_stride, dst.as_mut_ptr(), dst_stride, width, height);
                for h in 0..color_height {
                    for w in 0..width {
                        let offset = (dst_stride * h + w * 2) as usize;
                        // order: BRG
                        dst[offset + 0] = 0x03;
                        dst[offset + 1] = 0xc0;
                    }
                }
            }
            _ => {}
        }

        (dst, vec![dst_fmt.bytes_per_pixel() * width as usize])
    }
}
