#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(improper_ctypes)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/yuv_ffi.rs"));

#[cfg(not(target_os = "ios"))]
use crate::Frame;
use crate::{EncodeYuvFormat, TraitFrame};

#[cfg(feature = "hwcodec")]
pub mod hw {
    use crate::ImageFormat;
    use hbb_common::{anyhow::anyhow, ResultType};
    #[cfg(target_os = "windows")]
    use hwcodec::{ffmpeg::ffmpeg_linesize_offset_length, AVPixelFormat};

    #[cfg(target_os = "windows")]
    pub fn hw_nv12_to(
        fmt: ImageFormat,
        width: usize,
        height: usize,
        src_y: &[u8],
        src_uv: &[u8],
        src_stride_y: usize,
        src_stride_uv: usize,
        dst: &mut Vec<u8>,
        i420: &mut Vec<u8>,
        align: usize,
    ) -> ResultType<()> {
        let nv12_stride_y = src_stride_y;
        let nv12_stride_uv = src_stride_uv;
        if let Ok((linesize_i420, offset_i420, i420_len)) =
            ffmpeg_linesize_offset_length(AVPixelFormat::AV_PIX_FMT_YUV420P, width, height, align)
        {
            dst.resize(width * height * 4, 0);
            let i420_stride_y = linesize_i420[0];
            let i420_stride_u = linesize_i420[1];
            let i420_stride_v = linesize_i420[2];
            i420.resize(i420_len as _, 0);

            unsafe {
                let i420_offset_y = i420.as_ptr().add(0) as _;
                let i420_offset_u = i420.as_ptr().add(offset_i420[0] as _) as _;
                let i420_offset_v = i420.as_ptr().add(offset_i420[1] as _) as _;
                super::NV12ToI420(
                    src_y.as_ptr(),
                    nv12_stride_y as _,
                    src_uv.as_ptr(),
                    nv12_stride_uv as _,
                    i420_offset_y,
                    i420_stride_y,
                    i420_offset_u,
                    i420_stride_u,
                    i420_offset_v,
                    i420_stride_v,
                    width as _,
                    height as _,
                );
                match fmt {
                    ImageFormat::ARGB => {
                        super::I420ToARGB(
                            i420_offset_y,
                            i420_stride_y,
                            i420_offset_u,
                            i420_stride_u,
                            i420_offset_v,
                            i420_stride_v,
                            dst.as_mut_ptr(),
                            (width * 4) as _,
                            width as _,
                            height as _,
                        );
                    }
                    ImageFormat::ABGR => {
                        super::I420ToABGR(
                            i420_offset_y,
                            i420_stride_y,
                            i420_offset_u,
                            i420_stride_u,
                            i420_offset_v,
                            i420_stride_v,
                            dst.as_mut_ptr(),
                            (width * 4) as _,
                            width as _,
                            height as _,
                        );
                    }
                    _ => {
                        return Err(anyhow!("unsupported image format"));
                    }
                }
                return Ok(());
            };
        }
        return Err(anyhow!("get linesize offset failed"));
    }

    #[cfg(not(target_os = "windows"))]
    pub fn hw_nv12_to(
        fmt: ImageFormat,
        width: usize,
        height: usize,
        src_y: &[u8],
        src_uv: &[u8],
        src_stride_y: usize,
        src_stride_uv: usize,
        dst: &mut Vec<u8>,
        _i420: &mut Vec<u8>,
        _align: usize,
    ) -> ResultType<()> {
        dst.resize(width * height * 4, 0);
        unsafe {
            match fmt {
                ImageFormat::ARGB => {
                    match super::NV12ToARGB(
                        src_y.as_ptr(),
                        src_stride_y as _,
                        src_uv.as_ptr(),
                        src_stride_uv as _,
                        dst.as_mut_ptr(),
                        (width * 4) as _,
                        width as _,
                        height as _,
                    ) {
                        0 => Ok(()),
                        _ => Err(anyhow!("NV12ToARGB failed")),
                    }
                }
                ImageFormat::ABGR => {
                    match super::NV12ToABGR(
                        src_y.as_ptr(),
                        src_stride_y as _,
                        src_uv.as_ptr(),
                        src_stride_uv as _,
                        dst.as_mut_ptr(),
                        (width * 4) as _,
                        width as _,
                        height as _,
                    ) {
                        0 => Ok(()),
                        _ => Err(anyhow!("NV12ToABGR failed")),
                    }
                }
                _ => Err(anyhow!("unsupported image format")),
            }
        }
    }

    pub fn hw_i420_to(
        fmt: ImageFormat,
        width: usize,
        height: usize,
        src_y: &[u8],
        src_u: &[u8],
        src_v: &[u8],
        src_stride_y: usize,
        src_stride_u: usize,
        src_stride_v: usize,
        dst: &mut Vec<u8>,
    ) {
        let src_y = src_y.as_ptr();
        let src_u = src_u.as_ptr();
        let src_v = src_v.as_ptr();
        dst.resize(width * height * 4, 0);
        unsafe {
            match fmt {
                ImageFormat::ARGB => {
                    super::I420ToARGB(
                        src_y,
                        src_stride_y as _,
                        src_u,
                        src_stride_u as _,
                        src_v,
                        src_stride_v as _,
                        dst.as_mut_ptr(),
                        (width * 4) as _,
                        width as _,
                        height as _,
                    );
                }
                ImageFormat::ABGR => {
                    super::I420ToABGR(
                        src_y,
                        src_stride_y as _,
                        src_u,
                        src_stride_u as _,
                        src_v,
                        src_stride_v as _,
                        dst.as_mut_ptr(),
                        (width * 4) as _,
                        width as _,
                        height as _,
                    );
                }
                _ => {}
            }
        };
    }
}
#[cfg(not(target_os = "ios"))]
pub fn convert_to_yuv(
    captured: &Frame,
    dst_fmt: EncodeYuvFormat,
    dst: &mut Vec<u8>,
    mid_data: &mut Vec<u8>,
) {
    let src = captured.data();
    let src_stride = captured.stride();
    let captured_pixfmt = captured.pixfmt();
    match (captured_pixfmt, dst_fmt.pixfmt) {
        (crate::Pixfmt::BGRA, crate::Pixfmt::I420) | (crate::Pixfmt::RGBA, crate::Pixfmt::I420) => {
            let dst_stride_y = dst_fmt.stride[0];
            let dst_stride_uv = dst_fmt.stride[1];
            dst.resize(dst_fmt.h * dst_stride_y * 2, 0); // waste some memory to ensure memory safety
            let dst_y = dst.as_mut_ptr();
            let dst_u = dst[dst_fmt.u..].as_mut_ptr();
            let dst_v = dst[dst_fmt.v..].as_mut_ptr();
            let f = if captured_pixfmt == crate::Pixfmt::BGRA {
                ARGBToI420
            } else {
                ABGRToI420
            };
            unsafe {
                f(
                    src.as_ptr(),
                    src_stride[0] as _,
                    dst_y,
                    dst_stride_y as _,
                    dst_u,
                    dst_stride_uv as _,
                    dst_v,
                    dst_stride_uv as _,
                    dst_fmt.w as _,
                    dst_fmt.h as _,
                );
            }
        }
        (crate::Pixfmt::BGRA, crate::Pixfmt::NV12) | (crate::Pixfmt::RGBA, crate::Pixfmt::NV12) => {
            let dst_stride_y = dst_fmt.stride[0];
            let dst_stride_uv = dst_fmt.stride[1];
            dst.resize(dst_fmt.h * (dst_stride_y + dst_stride_uv / 2), 0);
            let dst_y = dst.as_mut_ptr();
            let dst_uv = dst[dst_fmt.u..].as_mut_ptr();
            let f = if captured_pixfmt == crate::Pixfmt::BGRA {
                ARGBToNV12
            } else {
                ABGRToNV12
            };
            unsafe {
                f(
                    src.as_ptr(),
                    src_stride[0] as _,
                    dst_y,
                    dst_stride_y as _,
                    dst_uv,
                    dst_stride_uv as _,
                    dst_fmt.w as _,
                    dst_fmt.h as _,
                );
            }
        }
        (crate::Pixfmt::BGRA, crate::Pixfmt::I444) | (crate::Pixfmt::RGBA, crate::Pixfmt::I444) => {
            let dst_stride_y = dst_fmt.stride[0];
            let dst_stride_u = dst_fmt.stride[1];
            let dst_stride_v = dst_fmt.stride[2];
            dst.resize(dst_fmt.h * (dst_stride_y + dst_stride_u + dst_stride_v), 0);
            let dst_y = dst.as_mut_ptr();
            let dst_u = dst[dst_fmt.u..].as_mut_ptr();
            let dst_v = dst[dst_fmt.v..].as_mut_ptr();
            unsafe {
                let src = if captured_pixfmt == crate::Pixfmt::BGRA {
                    src
                } else {
                    mid_data.resize(src.len(), 0);
                    ABGRToARGB(
                        src.as_ptr(),
                        src_stride[0] as _,
                        mid_data.as_mut_ptr(),
                        src_stride[0] as _,
                        dst_fmt.w as _,
                        dst_fmt.h as _,
                    );
                    mid_data
                };
                ARGBToI444(
                    src.as_ptr(),
                    src_stride[0] as _,
                    dst_y,
                    dst_stride_y as _,
                    dst_u,
                    dst_stride_u as _,
                    dst_v,
                    dst_stride_v as _,
                    dst_fmt.w as _,
                    dst_fmt.h as _,
                );
            }
        }
        _ => {}
    }
}
