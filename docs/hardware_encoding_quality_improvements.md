# Hardware Encoding Quality Improvements

## Problem Statement

At the same bitrate, hardware encoding currently uses less bandwidth than software encoding because the hardware encoder does not set min/max QP (Quantization Parameter) values. This results in poorer image quality with hardware encoding at lower FPS (e.g., 10 FPS), while the quality is quite good at higher FPS.

## Analysis

### Current Implementation (RustDesk)
- Uses hwcodec library from https://github.com/rustdesk-org/hwcodec
- Sets bitrate and basic encoder parameters
- Does NOT set:
  - Min/Max QP values for encoders
  - VBV (Video Buffering Verifier) buffer size
  - Encoder-specific quality options

### Reference Implementation (Sunshine)
Sunshine (https://github.com/LizardByte/Sunshine) maintains good image quality at 2 Mbps even at 10 FPS using:

1. **Min QP Controls** (prevents too-low QP which wastes bits on imperceptible quality):
   - H.264 NVENC: min_qp = 19
   - HEVC NVENC: min_qp = 23
   - AV1 NVENC: min_qp = 23

2. **VBV Buffer Size** for proper rate control:
   ```cpp
   // Single-frame VBV for better rate control
   ctx->rc_buffer_size = bitrate / framerate;
   ```

3. **Encoder-Specific Options**:
   - NVENC: Uses VBR with optional VBV percentage increase
   - QSV: Uses CBR-with-VBR mode (bit_rate = rc_max_rate - 1)
   - VAAPI: Uses VBR with single-frame VBV for Intel GPUs and AV1

## Required Changes

### 1. hwcodec Library Modifications

The hwcodec library needs to be modified to support:

#### A. Add QP parameters to EncodeContext structure (`src/ffmpeg_ram/encode.rs`)
```rust
pub struct EncodeContext {
    pub name: String,
    pub mc_name: Option<String>,
    pub width: i32,
    pub height: i32,
    pub pixfmt: AVPixelFormat,
    pub align: i32,
    pub fps: i32,
    pub gop: i32,
    pub rc: RateControl,
    pub quality: Quality,
    pub kbs: i32,
    pub q: i32,
    pub thread_count: i32,
    // NEW: Add QP controls
    pub min_qp: i32,  // -1 means not set
    pub max_qp: i32,  // -1 means not set
    pub rc_buffer_size: i32,  // -1 means not set, otherwise in kbps
}
```

#### B. Modify encoder initialization (`cpp/ffmpeg_ram/ffmpeg_ram_encode.cpp`)
Update the FFmpegRamEncoder constructor to accept and store new parameters:
```cpp
FFmpegRamEncoder(const char *name, const char *mc_name, int width, int height,
                 int pixfmt, int align, int fps, int gop, int rc, int quality,
                 int kbs, int q, int thread_count, int gpu, int min_qp, 
                 int max_qp, int rc_buffer_size, RamEncodeCallback callback)
```

#### C. Update `set_av_codec_ctx` function (`cpp/common/util.cpp`)
Add VBV buffer size calculation:
```cpp
void set_av_codec_ctx(AVCodecContext *c, const std::string &name, int kbs,
                      int gop, int fps, int rc_buffer_size) {
  // ... existing code ...
  
  // Set VBV buffer size for better rate control
  if (rc_buffer_size > 0) {
    c->rc_buffer_size = rc_buffer_size * 1000;
  } else if (kbs > 0) {
    // Default: single-frame VBV
    c->rc_buffer_size = (kbs * 1000) / fps;
  }
  
  // ... rest of existing code ...
}
```

#### D. Add QP setting function (`cpp/common/util.cpp`)
```cpp
bool set_qp_range(void *priv_data, const std::string &name, int min_qp, int max_qp) {
  int ret = 0;
  
  if (name.find("nvenc") != std::string::npos) {
    if (min_qp >= 0) {
      // For NVENC, set qmin for all frame types
      char qp_str[32];
      snprintf(qp_str, sizeof(qp_str), "%d,%d,%d", min_qp, min_qp, min_qp);
      if ((ret = av_opt_set(priv_data, "qmin", qp_str, 0)) < 0) {
        LOG_ERROR(std::string("nvenc set qmin failed, ret = ") + av_err2str(ret));
        return false;
      }
    }
    if (max_qp >= 0) {
      char qp_str[32];
      snprintf(qp_str, sizeof(qp_str), "%d,%d,%d", max_qp, max_qp, max_qp);
      if ((ret = av_opt_set(priv_data, "qmax", qp_str, 0)) < 0) {
        LOG_ERROR(std::string("nvenc set qmax failed, ret = ") + av_err2str(ret));
        return false;
      }
    }
  }
  
  if (name.find("qsv") != std::string::npos) {
    // QSV uses different option names
    if (min_qp >= 0) {
      if ((ret = av_opt_set_int(priv_data, "qmin", min_qp, 0)) < 0) {
        LOG_ERROR(std::string("qsv set qmin failed, ret = ") + av_err2str(ret));
        return false;
      }
    }
    if (max_qp >= 0) {
      if ((ret = av_opt_set_int(priv_data, "qmax", max_qp, 0)) < 0) {
        LOG_ERROR(std::string("qsv set qmax failed, ret = ") + av_err2str(ret));
        return false;
      }
    }
  }
  
  if (name.find("amf") != std::string::npos) {
    // AMF uses different option names
    if (min_qp >= 0) {
      if ((ret = av_opt_set_int(priv_data, "qp_min", min_qp, 0)) < 0) {
        LOG_ERROR(std::string("amf set qp_min failed, ret = ") + av_err2str(ret));
        // Don't fail, might not be supported
      }
    }
    if (max_qp >= 0) {
      if ((ret = av_opt_set_int(priv_data, "qp_max", max_qp, 0)) < 0) {
        LOG_ERROR(std::string("amf set qp_max failed, ret = ") + av_err2str(ret));
        // Don't fail, might not be supported
      }
    }
  }
  
  if (name.find("vaapi") != std::string::npos) {
    // VAAPI uses qp option (quality parameter)
    // For VAAPI, we typically use VBR mode instead of QP control
    // The qp parameter is only used in CQP mode
  }
  
  return true;
}
```

Call this function during encoder initialization in `ffmpeg_ram_encode.cpp`:
```cpp
// In FFmpegRamEncoder::init() after setting rate control
if (min_qp_ >= 0 || max_qp_ >= 0) {
  util_encode::set_qp_range(c_->priv_data, name_, min_qp_, max_qp_);
}
```

### 2. RustDesk Integration

#### A. Update `HwRamEncoderConfig` (`libs/scrap/src/common/hwcodec.rs`)
```rust
#[derive(Debug, Clone)]
pub struct HwRamEncoderConfig {
    pub name: String,
    pub mc_name: Option<String>,
    pub width: usize,
    pub height: usize,
    pub quality: f32,
    pub keyframe_interval: Option<usize>,
    pub fps: i32,  // NEW: Explicit FPS setting
}
```

#### B. Update encoder creation to use proper QP values
Modify the `HwRamEncoder::new` method to calculate appropriate QP values based on codec:
```rust
impl EncoderApi for HwRamEncoder {
    fn new(cfg: EncoderCfg, _i444: bool) -> ResultType<Self> {
        match cfg {
            EncoderCfg::HWRAM(config) => {
                let rc = Self::rate_control(&config);
                let mut bitrate = Self::bitrate(&config.name, config.width, config.height, config.quality);
                bitrate = Self::check_bitrate_range(&config, bitrate);
                let gop = config.keyframe_interval.unwrap_or(DEFAULT_GOP as _) as i32;
                let fps = config.fps.max(1);  // Ensure FPS is at least 1
                
                // Calculate QP values based on codec type (similar to Sunshine)
                let (min_qp, max_qp) = Self::calculate_qp_range(&config.name);
                
                // Calculate VBV buffer size (single-frame for low-latency)
                let rc_buffer_size = bitrate / (fps as u32);
                
                let ctx = EncodeContext {
                    name: config.name.clone(),
                    mc_name: config.mc_name.clone(),
                    width: config.width as _,
                    height: config.height as _,
                    pixfmt: DEFAULT_PIXFMT,
                    align: HW_STRIDE_ALIGN as _,
                    kbs: bitrate as i32,
                    fps,
                    gop,
                    quality: DEFAULT_HW_QUALITY,
                    rc,
                    q: -1,
                    thread_count: codec_thread_num(16) as _,
                    min_qp,
                    max_qp,
                    rc_buffer_size: rc_buffer_size as i32,
                };
                // ... rest of the code ...
            }
        }
    }
}

impl HwRamEncoder {
    // NEW: Calculate appropriate QP range based on encoder and codec
    fn calculate_qp_range(encoder_name: &str) -> (i32, i32) {
        // Based on Sunshine's implementation
        // These values prevent the encoder from using too-low QP (wasting bits)
        // while still allowing good quality
        
        if encoder_name.contains("nvenc") {
            if encoder_name.contains("h264") {
                return (19, -1);  // H.264: min QP 19, no max
            } else if encoder_name.contains("hevc") || encoder_name.contains("av1") {
                return (23, -1);  // HEVC/AV1: min QP 23, no max
            }
        } else if encoder_name.contains("qsv") {
            // QSV can also benefit from QP limits
            if encoder_name.contains("h264") {
                return (18, -1);
            } else if encoder_name.contains("hevc") {
                return (22, -1);
            }
        } else if encoder_name.contains("amf") {
            // AMD AMF encoder
            if encoder_name.contains("h264") {
                return (18, -1);
            } else if encoder_name.contains("hevc") {
                return (22, -1);
            }
        }
        
        // VAAPI and others: don't set QP limits (use VBR instead)
        (-1, -1)
    }
}
```

## Testing Plan

1. **Baseline Testing**:
   - Test current implementation at 1920×1080, 2 Mbps, 10 FPS
   - Measure visual quality (SSIM/VMAF) and actual bandwidth usage
   - Compare with software encoding

2. **After QP Changes**:
   - Test with min QP values set
   - Verify quality improvement at low FPS (10-15 FPS)
   - Ensure no regression at high FPS (30+ FPS)

3. **Multiple Encoders**:
   - Test NVENC (NVIDIA)
   - Test QSV (Intel)
   - Test AMF (AMD)
   - Test VAAPI (Linux)

4. **Multiple Resolutions**:
   - 1280×720 @ 1 Mbps
   - 1920×1080 @ 2 Mbps
   - 2560×1440 @ 4 Mbps

## Expected Results

- **At Low FPS (10-15 FPS)**: Significant quality improvement, especially in detailed areas and motion
- **At Medium FPS (20-25 FPS)**: Moderate quality improvement
- **At High FPS (30+ FPS)**: Minimal change (already good quality)
- **Bandwidth**: Should remain at target bitrate with better distribution across frames

## Implementation Steps

1. Fork hwcodec library and create feature branch
2. Implement changes to hwcodec as described above
3. Test hwcodec changes independently
4. Update rustdesk's Cargo.toml to use modified hwcodec
5. Update encoder configuration in rustdesk to pass FPS parameter
6. Test integration
7. Submit PR to hwcodec repository
8. Submit PR to rustdesk repository

## References

- Sunshine encoder implementation: https://github.com/LizardByte/Sunshine/blob/master/src/video.cpp
- FFmpeg rate control documentation: https://slhck.info/video/2017/03/01/rate-control.html
- NVENC programming guide: https://docs.nvidia.com/video-technologies/video-codec-sdk/nvenc-video-encoder-api-prog-guide/
- QSV documentation: https://www.intel.com/content/www/us/en/developer/articles/technical/quick-sync-video-and-ffmpeg-getting-started.html
