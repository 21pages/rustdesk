// This file shows the proposed changes to libs/scrap/src/common/hwcodec.rs
// to integrate with the improved hwcodec library

// Add to the imports section:
// (no changes needed to imports)

// Constants referenced in this example (defined elsewhere in actual implementation)
const DEFAULT_FPS: i32 = 30;  // Default frame rate if not specified
const DEFAULT_GOP: i32 = i32::MAX;  // Default GOP size (keyframe interval)

// Modified HwRamEncoderConfig structure to include FPS
#[derive(Debug, Clone)]
pub struct HwRamEncoderConfig {
    pub name: String,
    pub mc_name: Option<String>,
    pub width: usize,
    pub height: usize,
    pub quality: f32,
    pub keyframe_interval: Option<usize>,
    pub fps: i32,  // NEW: Explicit FPS parameter (was implicitly using DEFAULT_FPS=30)
}

// Add new helper methods to HwRamEncoder implementation
impl HwRamEncoder {
    /// Calculate appropriate QP (Quantization Parameter) range based on encoder and codec type.
    ///
    /// QP controls the amount of quantization applied during encoding. Lower QP = higher quality
    /// but also higher bitrate. The range is typically 0-51 for H.264, 0-51 for HEVC.
    ///
    /// # Purpose
    /// Setting a minimum QP prevents the encoder from wasting bits on imperceptible quality
    /// improvements. This is especially important at lower frame rates where the encoder
    /// might use very low QP values that don't provide visible quality gains.
    ///
    /// # Parameters
    /// * `encoder_name` - The name of the hardware encoder (e.g., "h264_nvenc", "hevc_qsv")
    ///
    /// # Returns
    /// A tuple of (min_qp, max_qp) where:
    /// - min_qp: Minimum QP value allowed (-1 means no minimum)
    /// - max_qp: Maximum QP value allowed (-1 means no maximum)
    ///
    /// # Values Based On
    /// These values are based on Sunshine's empirical testing and provide a good balance
    /// between quality and bitrate efficiency:
    /// - H.264: min_qp=18-19 (quality imperceptible below this)
    /// - HEVC: min_qp=22-23 (HEVC more efficient, can use higher QP)
    /// - AV1: min_qp=22-23 (most efficient codec)
    ///
    /// # References
    /// - Sunshine NVENC QP values: https://github.com/LizardByte/Sunshine/blob/master/src/nvenc/nvenc_config.h#L37-L44
    /// - FFmpeg QP documentation: https://trac.ffmpeg.org/wiki/Encode/H.264#crf
    fn calculate_qp_range(encoder_name: &str) -> (i32, i32) {
        // min_qp prevents the encoder from using too-low QP values
        // which waste bits on imperceptible quality improvements
        // This is especially important at lower frame rates
        
        if encoder_name.contains("nvenc") {
            // NVIDIA NVENC encoder
            // Reference: Sunshine NVENC configuration - https://github.com/LizardByte/Sunshine/blob/master/src/nvenc/nvenc_config.h#L37-L44
            if encoder_name.contains("h264") {
                // H.264: min QP 19 (Sunshine's value)
                // At QP < 19, quality improvements are barely perceptible
                // but bitrate increases significantly
                return (19, -1);  // -1 means no max QP limit
            } else if encoder_name.contains("hevc") {
                // HEVC: min QP 23 (Sunshine's value)
                // HEVC is more efficient, so can use slightly higher QP
                return (23, -1);
            } else if encoder_name.contains("av1") {
                // AV1: min QP 23 (Sunshine's value)
                return (23, -1);
            }
        } else if encoder_name.contains("qsv") {
            // Intel Quick Sync Video encoder
            // Reference: FFmpeg QSV encoder documentation - https://trac.ffmpeg.org/wiki/Hardware/QuickSync
            if encoder_name.contains("h264") {
                // QSV H.264: slightly lower than NVENC for compatibility
                return (18, -1);
            } else if encoder_name.contains("hevc") {
                // QSV HEVC
                return (22, -1);
            } else if encoder_name.contains("av1") {
                // QSV AV1 (if supported)
                return (22, -1);
            }
        } else if encoder_name.contains("amf") {
            // AMD AMF encoder
            // Reference: AMD AMF SDK - https://github.com/GPUOpen-LibrariesAndSDKs/AMF
            if encoder_name.contains("h264") {
                return (18, -1);
            } else if encoder_name.contains("hevc") {
                return (22, -1);
            }
        } else if encoder_name.contains("vaapi") {
            // VAAPI (Video Acceleration API) on Linux
            // VAAPI typically uses VBR mode with quality parameter
            // QP control only works in CQP mode, which we don't use for streaming
            // So we don't set QP limits for VAAPI
            // Reference: Sunshine VAAPI implementation - https://github.com/LizardByte/Sunshine/blob/master/src/platform/linux/vaapi.cpp#L423-L457
            return (-1, -1);
        } else if encoder_name.contains("videotoolbox") {
            // Apple VideoToolbox
            // VideoToolbox has its own quality control mechanism
            // Reference: Apple VideoToolbox documentation - https://developer.apple.com/documentation/videotoolbox
            return (-1, -1);
        }
        
        // Unknown encoder: don't set QP limits
        (-1, -1)
    }
    
    /// Calculate VBV (Video Buffering Verifier) buffer size for rate control.
    ///
    /// # Purpose
    /// VBV buffer size controls how much the encoder can deviate from the target bitrate
    /// over time. A smaller buffer means tighter bitrate control but may limit quality
    /// flexibility. A larger buffer allows more variation but can cause latency.
    ///
    /// # Why Single-Frame Buffering?
    /// For low-latency streaming (like remote desktop), we use single-frame VBV:
    /// - Minimizes latency (no multi-frame buffering)
    /// - Prevents large bitrate spikes
    /// - Provides smoother bitrate distribution
    /// - Improves quality at lower frame rates
    ///
    /// # Parameters
    /// * `bitrate_kbps` - Target bitrate in kilobits per second
    /// * `fps` - Frame rate in frames per second
    ///
    /// # Returns
    /// VBV buffer size in kilobits per second, or -1 if fps is invalid
    ///
    /// # Example
    /// At 2000 kbps and 10 FPS: buffer = 2000/10 = 200 kbps (25 KB per frame)
    /// At 2000 kbps and 30 FPS: buffer = 2000/30 = 66.7 kbps (8.3 KB per frame)
    ///
    /// This allows lower FPS to allocate more bits per frame, improving quality.
    ///
    /// # References
    /// - Sunshine VBV buffer calculation: https://github.com/LizardByte/Sunshine/blob/master/src/video.cpp#L1746
    /// - FFmpeg rate control guide: https://slhck.info/video/2017/03/01/rate-control.html
    fn calculate_rc_buffer_size(bitrate_kbps: u32, fps: i32) -> i32 {
        if fps <= 0 {
            return -1;  // Invalid FPS, let encoder decide
        }
        
        // Single-frame VBV buffer: allows one frame worth of bits
        // This provides:
        // 1. Low latency (no multi-frame buffering)
        // 2. Smooth bitrate distribution (prevents large spikes)
        // 3. Better quality at lower frame rates
        //
        // Example: At 2000 kbps and 10 FPS:
        //   Buffer = 2000 / 10 = 200 kbps = 25 KB per frame
        // Example: At 2000 kbps and 30 FPS:
        //   Buffer = 2000 / 30 = 66.7 kbps = 8.3 KB per frame
        //
        // Lower FPS gets more bits per frame, which improves quality
        // 
        // Note: fps > 0 is guaranteed by the early return above, preventing division by zero
        (bitrate_kbps / fps as u32) as i32
    }
}

// Modified EncoderApi::new implementation
impl EncoderApi for HwRamEncoder {
    fn new(cfg: EncoderCfg, _i444: bool) -> ResultType<Self>
    where
        Self: Sized,
    {
        match cfg {
            EncoderCfg::HWRAM(config) => {
                let rc = Self::rate_control(&config);
                let mut bitrate =
                    Self::bitrate(&config.name, config.width, config.height, config.quality);
                bitrate = Self::check_bitrate_range(&config, bitrate);
                let gop = config.keyframe_interval.unwrap_or(DEFAULT_GOP as _) as i32;
                
                // Use FPS from config (instead of hard-coded DEFAULT_FPS)
                let fps = if config.fps > 0 { config.fps } else { DEFAULT_FPS };
                
                // Calculate QP range based on encoder type
                let (min_qp, max_qp) = Self::calculate_qp_range(&config.name);
                
                // Calculate VBV buffer size for better rate control
                let rc_buffer_size = Self::calculate_rc_buffer_size(bitrate, fps);
                
                // Log the configuration for debugging
                log::info!(
                    "HwRamEncoder config: {}x{} @ {} FPS, {} kbps, QP range: [{}, {}], VBV buffer: {} kbps",
                    config.width, config.height, fps, bitrate,
                    min_qp, max_qp, rc_buffer_size
                );
                
                let ctx = EncodeContext {
                    name: config.name.clone(),
                    mc_name: config.mc_name.clone(),
                    width: config.width as _,
                    height: config.height as _,
                    pixfmt: DEFAULT_PIXFMT,
                    align: HW_STRIDE_ALIGN as _,
                    kbs: bitrate as i32,
                    fps,  // Use actual FPS instead of DEFAULT_FPS
                    gop,
                    quality: DEFAULT_HW_QUALITY,
                    rc,
                    q: -1,
                    thread_count: codec_thread_num(16) as _,
                    min_qp,           // NEW: Set minimum QP
                    max_qp,           // NEW: Set maximum QP
                    rc_buffer_size,   // NEW: Set VBV buffer size
                };
                
                let format = match Encoder::format_from_name(config.name.clone()) {
                    Ok(format) => format,
                    Err(_) => {
                        return Err(anyhow!(format!(
                            "failed to get format from name:{}",
                            config.name
                        )))
                    }
                };
                
                match Encoder::new(ctx.clone()) {
                    Ok(encoder) => Ok(HwRamEncoder {
                        encoder,
                        format,
                        pixfmt: ctx.pixfmt,
                        bitrate,
                        config,
                    }),
                    Err(_) => Err(anyhow!(format!("Failed to create encoder"))),
                }
            }
            _ => Err(anyhow!("encoder type mismatch")),
        }
    }
    
    // ... rest of the implementation remains the same ...
}

// Example of how to create encoder with new FPS parameter:
// (This would be in the code that creates the encoder, e.g., video_service.rs)
/*
let config = HwRamEncoderConfig {
    name: "h264_nvenc".to_string(),
    mc_name: None,
    width: 1920,
    height: 1080,
    quality: 1.0,
    keyframe_interval: None,
    fps: 10,  // NEW: Specify actual FPS (was implicitly 30)
};

let encoder = HwRamEncoder::new(EncoderCfg::HWRAM(config), false)?;
*/

// ============================================================================
// REFERENCES
// ============================================================================
//
// This implementation is based on analysis of Sunshine's encoder configuration:
//
// Primary References:
// - Sunshine source code: https://github.com/LizardByte/Sunshine
// - Sunshine NVENC config: https://github.com/LizardByte/Sunshine/blob/master/src/nvenc/nvenc_config.h
// - Sunshine NVENC implementation: https://github.com/LizardByte/Sunshine/blob/master/src/nvenc/nvenc_base.cpp
// - Sunshine video encoder: https://github.com/LizardByte/Sunshine/blob/master/src/video.cpp
// - Sunshine VAAPI implementation: https://github.com/LizardByte/Sunshine/blob/master/src/platform/linux/vaapi.cpp
//
// Hardware Encoder Documentation:
// - NVIDIA NVENC: https://docs.nvidia.com/video-technologies/video-codec-sdk/nvenc-video-encoder-api-prog-guide/
// - Intel QSV: https://www.intel.com/content/www/us/en/developer/articles/technical/quick-sync-video-and-ffmpeg-getting-started.html
// - AMD AMF: https://github.com/GPUOpen-LibrariesAndSDKs/AMF
// - Apple VideoToolbox: https://developer.apple.com/documentation/videotoolbox
//
// FFmpeg Documentation:
// - Rate control guide: https://slhck.info/video/2017/03/01/rate-control.html
// - H.264 encoding: https://trac.ffmpeg.org/wiki/Encode/H.264
// - QSV encoding: https://trac.ffmpeg.org/wiki/Hardware/QuickSync
//
// Technical Background:
// - QP (Quantization Parameter): Controls compression vs quality trade-off
// - VBV (Video Buffering Verifier): Controls bitrate variation over time
// - CBR (Constant Bitrate): Maintains fixed bitrate (less quality variation)
// - VBR (Variable Bitrate): Allows bitrate to vary for better quality
//
// Key Findings from Sunshine:
// - Min QP values prevent wasting bits on imperceptible quality at low FPS
// - Single-frame VBV (rc_buffer_size = bitrate / fps) improves low FPS quality
// - QSV CBR-with-VBR mode provides best quality for Intel encoders
// - VAAPI uses VBR mode instead of QP control for better results

