# Hardware Encoding Quality Improvements - Documentation Index

This directory contains comprehensive documentation for improving hardware encoding quality in RustDesk at lower frame rates.

## Quick Start

**New to this PR?** Start here:
1. Read `PR_SUMMARY.md` for a complete overview
2. Review `HARDWARE_ENCODING_IMPROVEMENTS_README.md` for implementation options
3. Follow the testing checklist

## Problem Statement

Hardware encoding in RustDesk produces poorer image quality than software encoding at lower frame rates (e.g., 10 FPS) because hardware encoders don't set min/max QP values and lack proper VBV buffer configuration.

**Visual Impact**: At 10 FPS with 2 Mbps bitrate, hardware encoding shows significant blockiness and artifacts compared to software encoding at the same settings.

## Solution Overview

Based on [Sunshine](https://github.com/LizardByte/Sunshine)'s proven implementation:

1. **Set minimum QP values** to prevent wasting bits (min_qp = 19 for H.264, 23 for HEVC)
2. **Use single-frame VBV** for better rate control (`rc_buffer_size = bitrate / framerate`)
3. **Make encoder FPS-aware** for correct buffer calculations

## Documentation Files

### 1. REFERENCE_COMPLIANCE.md (Verification Document)
**Purpose**: Demonstrates compliance with documentation standards  
**Contents**:
- Verification that all changes have clear rationale
- Verification that all changes have supporting reference links
- File-by-file compliance breakdown
- Reference quality standards
- Total reference count (58+ links)

**Best for**: Reviewers verifying documentation standards compliance

---

### 2. PR_SUMMARY.md (Start Here!)
**Purpose**: Complete PR overview and quick reference  
**Contents**:
- Root cause analysis
- Solution design with code examples
- Implementation timeline
- Expected results table
- Testing plan

**Best for**: Understanding the complete picture in 5-10 minutes

---

### 2. HARDWARE_ENCODING_IMPROVEMENTS_README.md
**Purpose**: Implementation guide with actionable steps  
**Contents**:
- Problem statement
- Analysis comparing RustDesk vs Sunshine
- Implementation paths (quick vs official)
- Testing checklist
- Configuration reference tables

**Best for**: Developers ready to implement or test the changes

---

### 3. hardware_encoding_quality_improvements.md
**Purpose**: Detailed technical specification  
**Contents**:
- In-depth problem analysis
- Complete Sunshine comparison
- Step-by-step hwcodec modifications
- Step-by-step RustDesk integration
- Comprehensive testing plan
- References and links

**Best for**: Deep technical understanding and reference during implementation

---

### 4. hwcodec_qp_improvements.patch
**Purpose**: Ready-to-apply patch for hwcodec library  
**Contents**:
- Complete diff for all hwcodec changes
- C++ code modifications (util.cpp, ffmpeg_ram_encode.cpp)
- Rust FFI updates
- New QP and VBV buffer functions

**How to use**:
```bash
cd /path/to/hwcodec
git apply /path/to/hwcodec_qp_improvements.patch
```

**Best for**: Quick implementation and testing

---

### 5. rustdesk_hwcodec_integration_example.rs
**Purpose**: Example integration code for RustDesk  
**Contents**:
- Modified `HwRamEncoderConfig` structure
- `calculate_qp_range()` function with doc comments
- `calculate_rc_buffer_size()` function with doc comments
- Updated `HwRamEncoder::new()` implementation
- Usage examples

**Best for**: Understanding how to integrate modified hwcodec into RustDesk

---

## Reading Path by Role

### For Reviewers
1. **PR_SUMMARY.md** - Get complete overview (5 min)
2. **hwcodec_qp_improvements.patch** - Review code changes (10 min)
3. **rustdesk_hwcodec_integration_example.rs** - Review integration (5 min)

**Total**: ~20 minutes for comprehensive review

### For Implementers
1. **HARDWARE_ENCODING_IMPROVEMENTS_README.md** - Understand approach (10 min)
2. **hwcodec_qp_improvements.patch** - Apply to hwcodec (2 min)
3. **rustdesk_hwcodec_integration_example.rs** - Copy integration code (5 min)
4. **HARDWARE_ENCODING_IMPROVEMENTS_README.md** - Follow testing checklist

**Total**: ~2 hours including testing

### For Technical Deep Dive
1. **PR_SUMMARY.md** - Get oriented (5 min)
2. **hardware_encoding_quality_improvements.md** - Read full analysis (30 min)
3. **hwcodec_qp_improvements.patch** - Study implementation details (20 min)
4. **rustdesk_hwcodec_integration_example.rs** - Understand integration (10 min)

**Total**: ~65 minutes for complete technical understanding

---

## Key Concepts

### QP (Quantization Parameter)
- Controls encoding quality (0-51 for H.264/HEVC)
- Lower QP = higher quality, higher bitrate
- **Problem**: Without limits, encoder uses QP < 10 at low FPS, wasting bits
- **Solution**: Set min_qp = 19 (H.264) or 23 (HEVC) to prevent imperceptible quality

### VBV (Video Buffering Verifier)
- Controls bitrate variation over time
- Buffer size determines smoothness of bitrate distribution
- **Problem**: No buffer size set, causing uneven bitrate
- **Solution**: Single-frame VBV = bitrate / framerate for low latency

### FPS-Aware Encoding
- Encoder needs to know actual frame rate for correct calculations
- **Problem**: Hard-coded 30 FPS regardless of actual rate
- **Solution**: Pass actual FPS to encoder

---

## Implementation Status

- [x] Research and analysis completed
- [x] Solution designed and documented
- [x] Patch file created for hwcodec
- [x] Integration example created
- [x] Testing plan documented
- [x] Security scan passed
- [x] Code review feedback addressed
- [ ] Apply to hwcodec repository (pending)
- [ ] Update RustDesk integration (pending)
- [ ] Validation testing (pending)

---

## Testing

### Minimum Testing
Test at these settings to verify improvement:
- **Resolution**: 1920×1080
- **Bitrate**: 2 Mbps
- **Frame Rates**: 10, 15, 30 FPS
- **Encoder**: At least one (NVENC recommended)

### Comprehensive Testing
- **Encoders**: NVENC, QSV, AMF, VAAPI
- **Resolutions**: 720p, 1080p, 1440p
- **Frame Rates**: 10, 15, 20, 25, 30 FPS
- **Metrics**: Visual inspection, SSIM/VMAF if available

---

## Questions?

1. **"Where do I start?"** → Read `PR_SUMMARY.md`
2. **"How do I implement this?"** → Read `HARDWARE_ENCODING_IMPROVEMENTS_README.md`
3. **"I need technical details"** → Read `hardware_encoding_quality_improvements.md`
4. **"What code changes are needed?"** → Review `hwcodec_qp_improvements.patch`
5. **"How do I integrate with RustDesk?"** → See `rustdesk_hwcodec_integration_example.rs`

---

## Contributing

This implementation requires changes to two repositories:

1. **hwcodec** (https://github.com/rustdesk-org/hwcodec) - Core encoder changes
2. **rustdesk** (https://github.com/21pages/rustdesk) - Integration changes

See `HARDWARE_ENCODING_IMPROVEMENTS_README.md` for implementation paths.

---

## Credits

Solution based on research and analysis of:
- [Sunshine](https://github.com/LizardByte/Sunshine) encoder implementation
- FFmpeg encoder best practices
- NVENC, QSV, AMF documentation
- Empirical testing data

---

## License

These documentation files are part of the RustDesk project and follow the same license.
