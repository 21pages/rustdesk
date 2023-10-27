#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(improper_ctypes)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/yuv_ffi.rs"));

#[cfg(not(target_os = "ios"))]
use crate::Frame;
use crate::{EncodeYuvFormat, TraitFrame};

use super::vpx::*;
use std::os::raw::c_int;
use vpx_img_fmt::{VPX_IMG_FMT_I420, VPX_IMG_FMT_I444};

// https://github.com/webmproject/libvpx/blob/master/vpx/src/vpx_image.c
#[inline]
fn get_vpx_stride(
    width: usize,
    height: usize,
    stride_align: usize,
    fmt: vpx_img_fmt,
) -> (usize, usize, Vec<usize>, usize, usize) {
    let mut img = Default::default();
    unsafe {
        vpx_img_wrap(
            &mut img,
            fmt,
            width as _,
            height as _,
            stride_align as _,
            0x1 as _,
        );
    }
    (
        img.w as _,
        img.h as _,
        img.stride.map(|s| s as usize).to_vec(),
        img.planes[1] as usize - img.planes[0] as usize,
        img.planes[2] as usize - img.planes[0] as usize,
    )
}

pub fn i420_to_rgb(width: usize, height: usize, src: &[u8], dst: &mut Vec<u8>) {
    let (_, _, src_stride, u, v) =
        get_vpx_stride(width, height, super::STRIDE_ALIGN, VPX_IMG_FMT_I420);
    let src_stride_y = src_stride[0];
    let src_stride_uv = src_stride[1];
    let src_y = src.as_ptr();
    let src_u = src[u..].as_ptr();
    let src_v = src[v..].as_ptr();
    dst.resize(width * height * 3, 0);
    unsafe {
        super::I420ToRAW(
            src_y,
            src_stride_y as _,
            src_u,
            src_stride_uv as _,
            src_v,
            src_stride_uv as _,
            dst.as_mut_ptr(),
            (width * 3) as _,
            width as _,
            height as _,
        );
    };
}

pub fn i420_to_bgra(width: usize, height: usize, src: &[u8], dst: &mut Vec<u8>) {
    let (_, _, src_stride, u, v) =
        get_vpx_stride(width, height, super::STRIDE_ALIGN, VPX_IMG_FMT_I420);
    let src_stride_y = src_stride[0];
    let src_stride_uv = src_stride[1];
    let src_y = src.as_ptr();
    let src_u = src[u..].as_ptr();
    let src_v = src[v..].as_ptr();
    dst.resize(width * height * 4, 0);
    unsafe {
        super::I420ToARGB(
            src_y,
            src_stride_y as _,
            src_u,
            src_stride_uv as _,
            src_v,
            src_stride_uv as _,
            dst.as_mut_ptr(),
            (width * 3) as _,
            width as _,
            height as _,
        );
    };
}

pub fn bgra_to_i420(width: usize, height: usize, src: &[u8], dst: &mut Vec<u8>) {
    let (_, h, dst_stride, u, v) =
        get_vpx_stride(width, height, super::STRIDE_ALIGN, VPX_IMG_FMT_I420);
    let dst_stride_y = dst_stride[0];
    let dst_stride_uv = dst_stride[1];
    dst.resize(h * dst_stride_y * 2, 0); // waste some memory to ensure memory safety
    let dst_y = dst.as_mut_ptr();
    let dst_u = dst[u..].as_mut_ptr();
    let dst_v = dst[v..].as_mut_ptr();
    unsafe {
        ARGBToI420(
            src.as_ptr(),
            (src.len() / height) as _,
            dst_y,
            dst_stride_y as _,
            dst_u,
            dst_stride_uv as _,
            dst_v,
            dst_stride_uv as _,
            width as _,
            height as _,
        );
    }
}

pub fn rgba_to_i420(width: usize, height: usize, src: &[u8], dst: &mut Vec<u8>) {
    let (_, h, dst_stride, u, v) =
        get_vpx_stride(width, height, super::STRIDE_ALIGN, VPX_IMG_FMT_I420);
    let dst_stride_y = dst_stride[0];
    let dst_stride_uv = dst_stride[1];
    dst.resize(h * dst_stride_y * 2, 0); // waste some memory to ensure memory safety
    let dst_y = dst.as_mut_ptr();
    let dst_u = dst[u..].as_mut_ptr();
    let dst_v = dst[v..].as_mut_ptr();
    unsafe {
        ABGRToI420(
            src.as_ptr(),
            (src.len() / height) as _,
            dst_y,
            dst_stride_y as _,
            dst_u,
            dst_stride_uv as _,
            dst_v,
            dst_stride_uv as _,
            width as _,
            height as _,
        );
    }
}

pub fn bgra_to_i444(width: usize, height: usize, src: &[u8], dst: &mut Vec<u8>) {
    let (_, h, dst_stride, u, v) =
        get_vpx_stride(width, height, super::STRIDE_ALIGN, VPX_IMG_FMT_I444);
    let dst_stride_y = dst_stride[0];
    let dst_stride_u = dst_stride[1];
    let dst_stride_v = dst_stride[2];
    dst.resize(h * (dst_stride_y + dst_stride_u + dst_stride_v), 0);
    let dst_y = dst.as_mut_ptr();
    let dst_u = dst[u..].as_mut_ptr();
    let dst_v = dst[v..].as_mut_ptr();
    unsafe {
        ARGBToI444(
            src.as_ptr(),
            (src.len() / height) as _,
            dst_y,
            dst_stride_y as _,
            dst_u,
            dst_stride_u as _,
            dst_v,
            dst_stride_v as _,
            width as _,
            height as _,
        );
    }
}

pub fn rgba_to_i444(width: usize, height: usize, src: &[u8], dst: &mut Vec<u8>, mid: &mut Vec<u8>) {
    let (_, h, dst_stride, u, v) =
        get_vpx_stride(width, height, super::STRIDE_ALIGN, VPX_IMG_FMT_I444);
    let dst_stride_y = dst_stride[0];
    let dst_stride_u = dst_stride[1];
    let dst_stride_v = dst_stride[2];
    dst.resize(h * (dst_stride_y + dst_stride_u + dst_stride_v), 0);
    mid.resize(dst.len(), 0);
    let dst_y = dst.as_mut_ptr();
    let dst_u = dst[u..].as_mut_ptr();
    let dst_v = dst[v..].as_mut_ptr();
    let src_stride_rgba = (src.len() / height) as _;
    unsafe {
        // B <-> R
        ARGBToABGR(
            src.as_ptr(),
            src_stride_rgba,
            mid.as_mut_ptr(),
            src_stride_rgba,
            width as _,
            height as _,
        );
        ARGBToI444(
            mid.as_ptr(),
            src_stride_rgba,
            dst_y,
            dst_stride_y as _,
            dst_u,
            dst_stride_u as _,
            dst_v,
            dst_stride_v as _,
            width as _,
            height as _,
        );
    }
}

pub unsafe fn nv12_to_i420(
    src_y: *const u8,
    src_stride_y: c_int,
    src_uv: *const u8,
    src_stride_uv: c_int,
    width: usize,
    height: usize,
    dst: &mut Vec<u8>,
) {
    let (_, h, dst_stride, u, v) =
        get_vpx_stride(width, height, super::STRIDE_ALIGN, VPX_IMG_FMT_I420);
    let dst_stride_y = dst_stride[0];
    let dst_stride_uv = dst_stride[1];
    dst.resize(h * dst_stride_y * 2, 0); // waste some memory to ensure memory safety
    let dst_y = dst.as_mut_ptr();
    let dst_u = dst[u..].as_mut_ptr();
    let dst_v = dst[v..].as_mut_ptr();
    NV12ToI420(
        src_y,
        src_stride_y,
        src_uv,
        src_stride_uv,
        dst_y,
        dst_stride_y as _,
        dst_u,
        dst_stride_uv as _,
        dst_v,
        dst_stride_uv as _,
        width as _,
        height as _,
    );
}

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
