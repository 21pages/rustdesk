use super::vpx::*;
use std::os::raw::c_int;

extern "C" {
    // seems libyuv uses reverse byte order compared with our view

    pub fn ARGBRotate(
        src_argb: *const u8,
        src_stride_argb: c_int,
        dst_argb: *mut u8,
        dst_stride_argb: c_int,
        src_width: c_int,
        src_height: c_int,
        mode: c_int,
    ) -> c_int;

    pub fn ARGBMirror(
        src_argb: *const u8,
        src_stride_argb: c_int,
        dst_argb: *mut u8,
        dst_stride_argb: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    pub fn ARGBToI420(
        src_bgra: *const u8,
        src_stride_bgra: c_int,
        dst_y: *mut u8,
        dst_stride_y: c_int,
        dst_u: *mut u8,
        dst_stride_u: c_int,
        dst_v: *mut u8,
        dst_stride_v: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    pub fn ABGRToI420(
        src_rgba: *const u8,
        src_stride_rgba: c_int,
        dst_y: *mut u8,
        dst_stride_y: c_int,
        dst_u: *mut u8,
        dst_stride_u: c_int,
        dst_v: *mut u8,
        dst_stride_v: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    pub fn ARGBToNV12(
        src_bgra: *const u8,
        src_stride_bgra: c_int,
        dst_y: *mut u8,
        dst_stride_y: c_int,
        dst_uv: *mut u8,
        dst_stride_uv: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    pub fn NV12ToI420(
        src_y: *const u8,
        src_stride_y: c_int,
        src_uv: *const u8,
        src_stride_uv: c_int,
        dst_y: *mut u8,
        dst_stride_y: c_int,
        dst_u: *mut u8,
        dst_stride_u: c_int,
        dst_v: *mut u8,
        dst_stride_v: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    // I420ToRGB24: RGB little endian (bgr in memory)
    // I420ToRaw: RGB big endian (rgb in memory) to RGBA.
    pub fn I420ToRAW(
        src_y: *const u8,
        src_stride_y: c_int,
        src_u: *const u8,
        src_stride_u: c_int,
        src_v: *const u8,
        src_stride_v: c_int,
        dst_rgba: *mut u8,
        dst_stride_raw: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    pub fn I420ToARGB(
        src_y: *const u8,
        src_stride_y: c_int,
        src_u: *const u8,
        src_stride_u: c_int,
        src_v: *const u8,
        src_stride_v: c_int,
        dst_rgba: *mut u8,
        dst_stride_rgba: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    pub fn I420ToABGR(
        src_y: *const u8,
        src_stride_y: c_int,
        src_u: *const u8,
        src_stride_u: c_int,
        src_v: *const u8,
        src_stride_v: c_int,
        dst_rgba: *mut u8,
        dst_stride_rgba: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    pub fn NV12ToARGB(
        src_y: *const u8,
        src_stride_y: c_int,
        src_uv: *const u8,
        src_stride_uv: c_int,
        dst_rgba: *mut u8,
        dst_stride_rgba: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    pub fn NV12ToABGR(
        src_y: *const u8,
        src_stride_y: c_int,
        src_uv: *const u8,
        src_stride_uv: c_int,
        dst_rgba: *mut u8,
        dst_stride_rgba: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;
}

// https://github.com/webmproject/libvpx/blob/master/vpx/src/vpx_image.c
#[inline]
fn get_vpx_i420_stride(
    width: usize,
    height: usize,
    stride_align: usize,
) -> (usize, usize, usize, usize, usize, usize) {
    let mut img = Default::default();
    unsafe {
        vpx_img_wrap(
            &mut img,
            vpx_img_fmt::VPX_IMG_FMT_I420,
            width as _,
            height as _,
            stride_align as _,
            0x1 as _,
        );
    }
    (
        img.w as _,
        img.h as _,
        img.stride[0] as _,
        img.stride[1] as _,
        img.planes[1] as usize - img.planes[0] as usize,
        img.planes[2] as usize - img.planes[0] as usize,
    )
}

pub fn i420_to_rgb(width: usize, height: usize, src: &[u8], dst: &mut Vec<u8>) {
    let (_, _, src_stride_y, src_stride_uv, u, v) =
        get_vpx_i420_stride(width, height, super::STRIDE_ALIGN);
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
    let (_, _, src_stride_y, src_stride_uv, u, v) =
        get_vpx_i420_stride(width, height, super::STRIDE_ALIGN);
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
    let (_, h, dst_stride_y, dst_stride_uv, u, v) =
        get_vpx_i420_stride(width, height, super::STRIDE_ALIGN);
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
    let (_, h, dst_stride_y, dst_stride_uv, u, v) =
        get_vpx_i420_stride(width, height, super::STRIDE_ALIGN);
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

pub unsafe fn nv12_to_i420(
    src_y: *const u8,
    src_stride_y: c_int,
    src_uv: *const u8,
    src_stride_uv: c_int,
    width: usize,
    height: usize,
    dst: &mut Vec<u8>,
) {
    let (_, h, dst_stride_y, dst_stride_uv, u, v) =
        get_vpx_i420_stride(width, height, super::STRIDE_ALIGN);
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
    use codec_common::PixelFormat;
    use hbb_common::{anyhow::bail, ResultType};

    fn align_size(size: i32, align: i32) -> i32 {
        (size + align - 1) / align * align
    }

    // https://github.com/obsproject/obs-studio/blob/712a6c3b331e0826a2a050633e6540937d80fe1a/libobs/media-io/video-frame.c#L23
    pub fn linesize_offset_length(
        pixfmt: PixelFormat,
        width: i32,
        height: i32,
    ) -> (Vec<i32>, Vec<i32>, i32) {
        match pixfmt {
            PixelFormat::NV12 => {
                let mut linesizes = vec![0, 0];
                let mut offsets = vec![0];

                let mut size = width * height;
                size = align_size(size, 32);
                offsets[0] = size;
                let cbcr_width = width + 1;
                size += cbcr_width * ((height + 1) / 2);
                size = align_size(size, 32);
                linesizes[0] = width;
                linesizes[1] = cbcr_width;

                (linesizes, offsets, size)
            }
            PixelFormat::I420 => (
                vec![width, width / 2, width / 2],
                vec![width * height, width * height * 5 / 4],
                width * height * 3 / 2,
            ),
        }
    }

    pub fn split_yuv(
        pixfmt: PixelFormat,
        width: i32,
        height: i32,
        yuv: &[u8],
    ) -> (Vec<&[u8]>, Vec<i32>) {
        let (linesizes, mut offsets, _) = linesize_offset_length(pixfmt, width, height);
        let offsets: Vec<usize> = offsets.drain(..).map(|e| e as usize).collect();
        match pixfmt {
            PixelFormat::NV12 => (vec![&yuv[0..offsets[0]], &yuv[offsets[0]..]], linesizes),
            PixelFormat::I420 => (
                vec![
                    &yuv[0..offsets[0]],
                    &yuv[offsets[0]..offsets[1]],
                    &yuv[offsets[1]..],
                ],
                linesizes,
            ),
        }
    }

    pub fn hw_bgra_to_i420(
        width: i32,
        height: i32,
        stride: &[i32],
        offset: &[i32],
        length: i32,
        src: &[u8],
        dst: &mut Vec<u8>,
    ) {
        let stride_y = stride[0];
        let stride_u = stride[1];
        let stride_v = stride[2];
        let offset_u = offset[0] as usize;
        let offset_v = offset[1] as usize;

        dst.resize(length as _, 0);
        let dst_y = dst.as_mut_ptr();
        let dst_u = dst[offset_u..].as_mut_ptr();
        let dst_v = dst[offset_v..].as_mut_ptr();
        unsafe {
            super::ARGBToI420(
                src.as_ptr(),
                src.len() as i32 / height,
                dst_y,
                stride_y,
                dst_u,
                stride_u,
                dst_v,
                stride_v,
                width,
                height,
            );
        }
    }

    pub fn hw_bgra_to_nv12(
        width: i32,
        height: i32,
        stride: &[i32],
        offset: &[i32],
        length: i32,
        src: &[u8],
        dst: &mut Vec<u8>,
    ) {
        let stride_y = stride[0];
        let stride_uv = stride[1];
        let offset_uv = offset[0] as usize;
        dst.resize(length as _, 0);
        let dst_y = dst.as_mut_ptr();
        let dst_uv = dst[offset_uv..].as_mut_ptr();
        unsafe {
            super::ARGBToNV12(
                src.as_ptr(),
                src.len() as i32 / height,
                dst_y,
                stride_y,
                dst_uv,
                stride_uv,
                width,
                height,
            );
        }
    }

    #[cfg(target_os = "windows")]
    pub fn hw_nv12_to(
        fmt: ImageFormat,
        width: i32,
        height: i32,
        src_y: &[u8],
        src_uv: &[u8],
        src_stride_y: i32,
        src_stride_uv: i32,
        dst: &mut Vec<u8>,
        i420: &mut Vec<u8>,
        _align: i32,
    ) -> ResultType<()> {
        let nv12_stride_y = src_stride_y;
        let nv12_stride_uv = src_stride_uv;
        let (i420_strides, i420_offsets, i420_len) =
            linesize_offset_length(PixelFormat::I420, width, height);
        dst.resize((width * height * 4) as usize, 0);
        i420.resize(i420_len as usize, 0);

        unsafe {
            let i420_offset_y = i420.as_ptr().add(0) as _;
            let i420_offset_u = i420.as_ptr().add(i420_offsets[0] as usize) as _;
            let i420_offset_v = i420.as_ptr().add(i420_offsets[1] as usize) as _;
            if 0 != super::NV12ToI420(
                src_y.as_ptr(),
                nv12_stride_y,
                src_uv.as_ptr(),
                nv12_stride_uv,
                i420_offset_y,
                i420_strides[0],
                i420_offset_u,
                i420_strides[1],
                i420_offset_v,
                i420_strides[2],
                width,
                height,
            ) {
                bail!("NV12ToI420 failed");
            }
            hw_i420_to(
                fmt,
                width,
                height,
                i420_offset_y,
                i420_offset_u,
                i420_offset_v,
                i420_strides[0],
                i420_strides[1],
                i420_strides[2],
                dst,
            );
        };
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn hw_nv12_to(
        fmt: ImageFormat,
        width: i32,
        height: i32,
        src_y: &[u8],
        src_uv: &[u8],
        src_stride_y: i32,
        src_stride_uv: i32,
        dst: &mut Vec<u8>,
        _i420: &mut Vec<u8>,
        _align: i32,
    ) -> ResultType<()> {
        dst.resize((width * height * 4) as usize, 0);
        unsafe {
            match fmt {
                ImageFormat::ARGB => {
                    match super::NV12ToARGB(
                        src_y.as_ptr(),
                        src_stride_y as _,
                        src_uv.as_ptr(),
                        src_stride_uv as _,
                        dst.as_mut_ptr(),
                        width * 4,
                        width,
                        height,
                    ) {
                        0 => Ok(()),
                        _ => bail!("NV12ToARGB failed"),
                    }
                }
                ImageFormat::ABGR => {
                    match super::NV12ToABGR(
                        src_y.as_ptr(),
                        src_stride_y as _,
                        src_uv.as_ptr(),
                        src_stride_uv as _,
                        dst.as_mut_ptr(),
                        width * 4,
                        width,
                        height,
                    ) {
                        0 => Ok(()),
                        _ => bail!("NV12ToABGR failed"),
                    }
                }
                _ => bail!("unsupported image format"),
            }
        }
    }

    pub fn hw_i420_to(
        fmt: ImageFormat,
        width: i32,
        height: i32,
        src_y: *const u8,
        src_u: *const u8,
        src_v: *const u8,
        src_stride_y: i32,
        src_stride_u: i32,
        src_stride_v: i32,
        dst: &mut Vec<u8>,
    ) {
        dst.resize((width * height * 4) as usize, 0);
        unsafe {
            match fmt {
                ImageFormat::ARGB => {
                    super::I420ToARGB(
                        src_y,
                        src_stride_y,
                        src_u,
                        src_stride_u,
                        src_v,
                        src_stride_v,
                        dst.as_mut_ptr(),
                        (width * 4) as _,
                        width as _,
                        height as _,
                    );
                }
                ImageFormat::ABGR => {
                    super::I420ToABGR(
                        src_y,
                        src_stride_y,
                        src_u,
                        src_stride_u,
                        src_v,
                        src_stride_v,
                        dst.as_mut_ptr(),
                        width * 4,
                        width,
                        height,
                    );
                }
                _ => {}
            }
        };
    }
}
