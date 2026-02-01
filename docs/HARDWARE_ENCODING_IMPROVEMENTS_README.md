# Hardware Encoding Quality Improvements - Implementation Guide

## Overview

This PR addresses the issue where hardware encoding in RustDesk produces poorer image quality than software encoding at lower frame rates (e.g., 10 FPS) at the same bitrate.

## Problem

The root cause is that hardware encoders don't set min/max QP (Quantization Parameter) values and lack proper VBV (Video Buffering Verifier) buffer size configuration. This causes:

- At low FPS (10 FPS): Poor quality - encoder uses very low QP values, wasting bits on imperceptible quality
- At high FPS (30+ FPS): Good quality - natural QP distribution works well

## Solution

Based on analysis of [Sunshine](https://github.com/LizardByte/Sunshine), which maintains excellent quality at 2 Mbps @ 1920×1080 even at 10 FPS, we need to:

1. Set minimum QP values per codec type (prevents wasting bits)
2. Use single-frame VBV buffer (better rate control at low FPS)
3. Make encoder FPS-aware for proper calculations

## Files in This PR

### 1. `docs/hardware_encoding_quality_improvements.md`
Comprehensive analysis including:
- Problem statement and root cause analysis
- Comparison with Sunshine implementation
- Detailed technical specifications
- Expected results and testing plan

### 2. `docs/hwcodec_qp_improvements.patch`
Patch file with all necessary changes to the hwcodec library:
- Adds `min_qp`, `max_qp`, `rc_buffer_size` parameters to `EncodeContext`
- Implements `set_qp_range()` function with encoder-specific logic
- Updates VBV buffer size calculation in `set_av_codec_ctx()`
- Adds proper parameter propagation through C++/Rust FFI

### 3. `docs/rustdesk_hwcodec_integration_example.rs`
Example code showing how to integrate with modified hwcodec:
- Modified `HwRamEncoderConfig` with FPS parameter
- `calculate_qp_range()` function with values for each encoder
- `calculate_rc_buffer_size()` for VBV buffer calculation
- Updated `HwRamEncoder::new()` implementation

## Implementation Path

### Option A: Quick Implementation (Recommended for Testing)
1. Apply the patch to a local fork of hwcodec
2. Update RustDesk's `Cargo.toml` to point to the forked hwcodec:
   ```toml
   [dependencies.hwcodec]
   git = "https://github.com/YOUR_USERNAME/hwcodec"
   branch = "qp-improvements"
   optional = true
   ```
3. Update encoder creation code in RustDesk to pass FPS
4. Test and measure quality improvements

### Option B: Official Implementation
1. Submit PR to rustdesk-org/hwcodec with the patch changes
2. Wait for review and merge
3. Update RustDesk to use new hwcodec version
4. Update RustDesk integration code
5. Test and verify

## Expected Quality Improvements

### Before Changes
- **10 FPS @ 2 Mbps**: Poor quality, blocky in detailed areas
- **30 FPS @ 2 Mbps**: Good quality

### After Changes
- **10 FPS @ 2 Mbps**: Much improved quality, comparable to software encoding
- **30 FPS @ 2 Mbps**: Similar or slightly better quality
- Bitrate: Stays at target (no change)
- Latency: No impact (still using single-frame encoding)

## Testing Checklist

- [ ] Test NVENC (NVIDIA) on Windows
- [ ] Test QSV (Intel) on Windows/Linux
- [ ] Test AMF (AMD) on Windows  
- [ ] Test VAAPI on Linux
- [ ] Test at 10 FPS, 15 FPS, 20 FPS, 30 FPS
- [ ] Test at 720p, 1080p, 1440p resolutions
- [ ] Measure SSIM/VMAF if possible
- [ ] Verify bandwidth stays at target
- [ ] Check for any visual artifacts

## Configuration Per Encoder

Based on Sunshine's proven configuration:

| Encoder | Codec | Min QP | Max QP | Notes |
|---------|-------|--------|--------|-------|
| NVENC | H.264 | 19 | -1 | Prevents QP < 19 (imperceptible quality) |
| NVENC | HEVC | 23 | -1 | HEVC more efficient, higher min QP |
| NVENC | AV1 | 23 | -1 | AV1 most efficient |
| QSV | H.264 | 18 | -1 | Intel slightly lower for compatibility |
| QSV | HEVC | 22 | -1 | |
| AMF | H.264 | 18 | -1 | AMD encoder |
| AMF | HEVC | 22 | -1 | |
| VAAPI | All | -1 | -1 | Uses VBR quality parameter instead |
| VideoToolbox | All | -1 | -1 | macOS has own quality control |

## VBV Buffer Size

Formula: `rc_buffer_size = bitrate / framerate`

Examples:
- 2000 kbps @ 10 FPS = 200 kbps buffer (25 KB per frame)
- 2000 kbps @ 30 FPS = 66.7 kbps buffer (8.3 KB per frame)

This allows lower FPS to use more bits per frame, improving quality.

## Questions?

Contact the maintainers or reference:
- Sunshine implementation: https://github.com/LizardByte/Sunshine/blob/master/src/video.cpp
- FFmpeg rate control guide: https://slhck.info/video/2017/03/01/rate-control.html
- NVENC programming guide: https://docs.nvidia.com/video-technologies/video-codec-sdk/

## Related Issues

This addresses the root cause mentioned in the GitHub task:
> "At the same bitrate, hardware encoding currently uses less bandwidth than software encoding because the hardware encoder does not set min/max QP values."
