use super::*;
use std::time::{Duration, Instant};

// Constants
pub const FPS: u32 = 30;
pub const MIN_FPS: u32 = 1;
pub const MAX_FPS: u32 = 120;

// Bitrate ratio constants for different quality levels
const BR_MAX: f32 = 40.0;
const BR_MIN: f32 = 0.2;
const BR_BEST: f32 = 1.5;
const BR_BALANCED: f32 = 1.0;
const BR_SPEED: f32 = 0.67;

const HISTORY_CAPTURE_TIMES_LEN: usize = 30;

// Refresh type for QoS updates
#[derive(Debug, PartialEq, Eq)]
pub enum RefreshType {
    SetImageQuality,
    FPS,
    All,
}

// Quality-related types and implementations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quality {
    Best,
    Balanced,
    Low,
    Custom(f32),
}

impl Default for Quality {
    fn default() -> Self {
        Self::Balanced
    }
}

impl Quality {
    pub fn is_custom(&self) -> bool {
        matches!(self, Quality::Custom(_))
    }

    pub fn ratio(&self) -> f32 {
        match self {
            Quality::Best => BR_BEST,
            Quality::Balanced => BR_BALANCED,
            Quality::Low => BR_SPEED,
            Quality::Custom(v) => *v,
        }
    }

    // Minimum FPS requirements for different quality levels
    fn min_fps(&self) -> u32 {
        match self {
            Quality::Best => 10,      // Higher quality tolerates lower FPS
            Quality::Balanced => 15,  // Standard FPS for balanced mode
            Quality::Low => 18,       // Low quality prioritizes smoothness
            Quality::Custom(_) => 15, // Default target for custom quality
        }
    }

    fn proper_fps(&self) -> u32 {
        match self {
            Quality::Best => 15,      // Higher quality tolerates lower FPS
            Quality::Balanced => 20,  // Standard FPS for balanced mode
            Quality::Low => 22,       // Low quality prioritizes smoothness
            Quality::Custom(_) => 20, // Default target for custom quality
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum DelayState {
    Normal = 0,
    LowDelay = 200,
    HighDelay = 500,
    Broken = 1000,
}

impl Default for DelayState {
    fn default() -> Self {
        DelayState::Normal
    }
}

impl DelayState {
    fn from_delay(delay: u32) -> Self {
        if delay > DelayState::Broken as u32 {
            DelayState::Broken
        } else if delay > DelayState::HighDelay as u32 {
            DelayState::HighDelay
        } else if delay > DelayState::LowDelay as u32 {
            DelayState::LowDelay
        } else {
            DelayState::Normal
        }
    }
}

#[derive(Default, Debug, Copy, Clone)]
struct Delay {
    state: DelayState,
    staging_state: DelayState,
    delay: u32,
    counter: u32,
    slower_than_old_state: Option<bool>,
}

// User session data structure
#[derive(Default, Debug, Clone)]
struct UserData {
    support_video_ack: bool,
    auto_adjust_fps: Option<u32>, // reserve for compatibility
    custom_fps: Option<u32>,
    quality: Option<(i64, Quality)>, // (timestamp, quality)
    record: bool,
    congested: bool,
    bandwidth: f32,
    delay: Option<Delay>,
    response_delayed: bool,
}

// Main QoS controller structure
pub struct VideoQoS {
    fps: u32,
    ratio: f32,
    users: HashMap<i32, UserData>,
    bitrate_store: u32,
    support_abr: HashMap<usize, bool>,
    start: Instant,
    target_fps: u32,
    congested_in_one_second: u32,
    history_capture_times: Vec<u32>,
    frame_count_since_adjust_ratio: usize,
    is_hardware: bool,
}

impl Default for VideoQoS {
    fn default() -> Self {
        VideoQoS {
            fps: FPS,
            ratio: BR_BALANCED,
            users: Default::default(),
            bitrate_store: 0,
            support_abr: Default::default(),
            start: Instant::now(),
            congested_in_one_second: 0,
            history_capture_times: Vec::new(),
            target_fps: FPS,
            frame_count_since_adjust_ratio: 0,
            is_hardware: false,
        }
    }
}

// VideoQoS implementation - Basic functionality
impl VideoQoS {
    // Calculate seconds per frame based on current FPS
    pub fn spf(&self) -> Duration {
        Duration::from_secs_f32(1. / (self.fps() as f32))
    }

    // Get current FPS within valid range
    pub fn fps(&self) -> u32 {
        if self.fps >= MIN_FPS && self.fps <= MAX_FPS {
            self.fps
        } else {
            FPS
        }
    }

    // Get current bitrate ratio with bounds checking
    pub fn ratio(&mut self) -> f32 {
        if self.ratio < BR_MIN || self.ratio > BR_MAX {
            self.ratio = BR_BALANCED;
        }
        self.ratio
    }

    // Check if any user is in recording mode
    pub fn record(&self) -> bool {
        self.users.iter().any(|u| u.1.record)
    }

    pub fn set_support_abr(&mut self, display_idx: usize, support: bool) {
        self.support_abr.insert(display_idx, support);
    }

    // Check if variable bitrate encoding is supported and enabled
    pub fn in_vbr_state(&self) -> bool {
        Config::get_option("enable-abr") != "N" && self.support_abr.iter().all(|e| *e.1)
    }

    // Store bitrate for later use
    pub fn store_bitrate(&mut self, bitrate: u32) {
        self.bitrate_store = bitrate;
    }

    // Get stored bitrate
    pub fn bitrate(&self) -> u32 {
        self.bitrate_store
    }
}

// VideoQoS implementation - Congestion control and FPS adjustment
impl VideoQoS {
    // Initialize timing for congestion monitoring
    pub fn start(&mut self, is_hardware: bool) {
        self.start = Instant::now();
        self.is_hardware = is_hardware;
    }

    // Main congestion control function
    pub fn congested(&mut self, sent_frame_sizes: &mut Vec<u64>) -> bool {
        let congested = self.users.iter().any(|u| u.1.congested);
        if congested {
            self.congested_in_one_second += 1;
        }

        let sent_frame_one_second = sent_frame_sizes.len();
        let dynamic_screen = sent_frame_one_second > 0; // TODO: check if screen is dynamic, maybe always congested
        let elapsed = self.start.elapsed().as_millis();

        // Process metrics every second
        if elapsed > 1000 {
            self.handle_one_second_elapsed(dynamic_screen, sent_frame_one_second, sent_frame_sizes);
        }

        // Adjust quality ratio if enough samples collected
        self.process_history_capture_times(dynamic_screen);

        congested
    }

    // Process metrics collected over one second
    fn handle_one_second_elapsed(
        &mut self,
        dynamic_screen: bool,
        sent_frame_one_second: usize,
        sent_frame_sizes: &mut Vec<u64>,
    ) {
        log::info!(
            "congested: {}, fps: {}, sent_frame_size: {:?}, payload: {:?}, ratio: {:.2}",
            self.congested_in_one_second,
            self.fps,
            sent_frame_one_second,
            sent_frame_sizes.iter().sum::<u64>(),
            self.ratio,
        );

        if dynamic_screen {
            self.fps = self.congested_fps();
        }

        self.start = Instant::now();
        self.congested_in_one_second = 0;
        self.frame_count_since_adjust_ratio += sent_frame_one_second;
        sent_frame_sizes.clear();
    }

    // Process historical capture times and adjust ratio if needed
    fn process_history_capture_times(&mut self, dynamic_screen: bool) {
        if self.history_capture_times.len() >= 6 {
            let avg = self.history_capture_times.iter().sum::<u32>() as f32
                / self.history_capture_times.len() as f32;
            self.history_capture_times.clear();

            // Avoid too frequent adjustments to prevent image blur
            if dynamic_screen && self.frame_count_since_adjust_ratio > 30 {
                self.frame_count_since_adjust_ratio = 0;
                self.adjust_ratio(avg);
            }
        }
    }

    // Calculate FPS based on congestion status
    #[inline]
    fn congested_fps(&mut self) -> u32 {
        let capture_times = if self.fps > self.congested_in_one_second {
            self.fps - self.congested_in_one_second
        } else {
            0
        };
        rm_first(
            &mut self.history_capture_times,
            capture_times,
            HISTORY_CAPTURE_TIMES_LEN,
        );
        if capture_times < self.target_fps {
            return std::cmp::min(self.target_fps, capture_times + 2);
        }
        self.fps
    }
}

// VideoQoS implementation - Quality adjustment
impl VideoQoS {
    // Adjust quality ratio based on performance metrics
    fn adjust_ratio(&mut self, avg_fps: f32) {
        let target_quality = self.lastest_quality();
        let target_ratio = target_quality.ratio();
        let (min, max) = (BR_MIN, BR_MAX.min(target_ratio * 3.0));
        let fps_ratio = avg_fps / self.target_fps as f32;
        let current_ratio = self.ratio;
        let mut v = self.ratio;

        log::info!(
            "adjust_ratio: target_quality: {:?}, target_ratio: {:?}, min: {:?}, max: {:?}, fps_ratio: {:?}, current_ratio: {:?}, avg_fps: {:.1}",
            target_quality,
            target_ratio,
            min,
            max,
            fps_ratio,
            current_ratio,
            avg_fps
        );

        // Basic guarantees for any quality mode
        if self.target_fps > 20 && avg_fps < 10.0 {
            // When target_fps > 20, ensure fps not lower than 10
            v = current_ratio * 0.7; // Aggressive quality reduction
        } else {
            match target_quality {
                Quality::Best => {
                    // Prioritize quality, allow slightly lower FPS
                    if current_ratio > BR_BEST {
                        if fps_ratio > 0.8 {
                            v = current_ratio * 1.1;
                        } else if fps_ratio < 0.5 {
                            v = current_ratio * 0.8;
                        }
                    } else {
                        if fps_ratio > 0.7 {
                            v = current_ratio * 1.2;
                        } else if fps_ratio < 0.4 {
                            v = current_ratio * 0.9;
                        }
                    }
                }
                Quality::Balanced => {
                    // Balance between quality and FPS
                    if current_ratio > BR_BEST {
                        if fps_ratio > 0.9 {
                            v = current_ratio * 1.1;
                        } else if fps_ratio < 0.7 {
                            v = current_ratio * 0.8;
                        }
                    } else if current_ratio > BR_BALANCED {
                        if fps_ratio > 0.85 {
                            v = current_ratio * 1.1;
                        } else if fps_ratio < 0.6 {
                            v = current_ratio * 0.9;
                        }
                    } else {
                        if fps_ratio > 0.8 {
                            v = current_ratio * 1.15;
                        } else if fps_ratio < 0.5 {
                            v = current_ratio * 0.85;
                        }
                    }
                }
                Quality::Low => {
                    // Prioritize FPS, accept lower quality
                    if current_ratio > BR_BEST {
                        if fps_ratio > 0.95 {
                            v = current_ratio * 1.05;
                        } else if fps_ratio < 0.8 {
                            v = current_ratio * 0.8;
                        }
                    } else if current_ratio > BR_BALANCED {
                        if fps_ratio > 0.9 {
                            v = current_ratio * 1.05;
                        } else if fps_ratio < 0.7 {
                            v = current_ratio * 0.85;
                        }
                    } else {
                        if fps_ratio > 0.85 {
                            v = current_ratio * 1.1;
                        } else if fps_ratio < 0.6 {
                            v = current_ratio * 0.8;
                        }
                    }
                }
                Quality::Custom(_) => {}
            }
        }

        // Apply minimum ratio guarantees based on FPS
        if avg_fps > 15.0 {
            v = v.max(BR_BALANCED);
        } else if avg_fps > 10.0 {
            v = v.max(BR_SPEED);
        }

        // Final clamp within allowed range
        self.ratio = v.clamp(min, max);

        log::info!(
            "after adjust - ratio: {:.2}, fps_ratio: {:.2}, quality: {:?}, avg_fps: {:.1}",
            self.ratio,
            fps_ratio,
            target_quality,
            avg_fps
        );
    }

    // Adjust ratio when below target quality
    fn adjust_ratio_below_target(&mut self, fps_ratio: f32, quality_ratio: f32, min: f32) {
        let step_num = 4.0;
        if fps_ratio >= 1.1 {
            // FPS is sufficient, increase quality
            let mut advance = (quality_ratio - self.ratio) / step_num;
            if advance < 0.15 {
                advance = 0.15;
            }
            self.ratio += advance;
        } else if fps_ratio < 0.9 {
            // FPS is insufficient, decrease quality
            let mut advance = (self.ratio - min) / step_num;
            if advance < 0.15 {
                advance = 0.15;
            }
            self.ratio -= advance;
        }
        // Keep current ratio if fps_ratio is between 0.9 and 1.1
    }

    // Adjust ratio when above target quality
    fn adjust_ratio_above_target(&mut self, fps_ratio: f32, quality_ratio: f32, max: f32) {
        let step_num = 4.0;
        if fps_ratio >= 1.2 {
            // FPS is very sufficient, try to increase quality further
            let mut advance = (max - self.ratio) / (step_num * 2.0); // More conservative step
            if advance < 0.1 {
                advance = 0.1;
            }
            self.ratio += advance;
        } else if fps_ratio < 0.95 {
            // FPS is slightly insufficient, decrease quality
            let mut advance = (self.ratio - quality_ratio) / step_num;
            if advance < 0.15 {
                advance = 0.15;
            }
            self.ratio -= advance;
        }
        // Keep current ratio if fps_ratio is between 0.95 and 1.2
    }

    // Get latest quality settings from all users
    #[inline]
    fn lastest_quality(&self) -> Quality {
        self.users
            .iter()
            .map(|(_, u)| u.quality)
            .filter(|q| *q != None)
            .max_by(|a, b| a.unwrap_or_default().0.cmp(&b.unwrap_or_default().0))
            .flatten()
            .unwrap_or((0, Quality::Balanced))
            .1
    }
}

// VideoQoS implementation - User session management
impl VideoQoS {
    // Initialize new user session
    pub fn on_connection_open(&mut self, id: i32, support_video_ack: bool) {
        self.users.insert(
            id,
            UserData {
                support_video_ack,
                ..Default::default()
            },
        );
        self.refresh(RefreshType::All);
    }

    // Clean up user session
    pub fn on_connection_close(&mut self, id: i32) {
        self.users.remove(&id);
        if self.users.is_empty() {
            *self = Default::default();
        }
        self.refresh(RefreshType::All);
    }

    pub fn user_custom_fps(&mut self, id: i32, fps: u32) {
        if fps < MIN_FPS {
            return;
        }
        if let Some(user) = self.users.get_mut(&id) {
            user.custom_fps = Some(fps);
            self.refresh(RefreshType::FPS);
        }
    }

    pub fn user_auto_adjust_fps(&mut self, id: i32, fps: u32) {
        if let Some(user) = self.users.get_mut(&id) {
            user.auto_adjust_fps = Some(fps);
            self.refresh(RefreshType::FPS);
        }
    }

    pub fn user_image_quality(&mut self, id: i32, image_quality: i32) {
        let quality = Some((hbb_common::get_time(), self.convert_quality(image_quality)));
        if let Some(user) = self.users.get_mut(&id) {
            log::info!("user.quality: {:?}", user.quality);
            user.quality = quality;
            self.refresh(RefreshType::SetImageQuality);
        }
    }

    pub fn user_network_delay(&mut self, id: i32, delay: u32) {
        let state = DelayState::from_delay(delay);
        let debounce = 3;
        if let Some(user) = self.users.get_mut(&id) {
            if let Some(d) = &mut user.delay {
                d.delay = (delay + d.delay) / 2;
                let new_state = DelayState::from_delay(d.delay);
                let slower_than_old_state = new_state as i32 - d.staging_state as i32;
                let slower_than_old_state = if slower_than_old_state > 0 {
                    Some(true)
                } else if slower_than_old_state < 0 {
                    Some(false)
                } else {
                    None
                };
                if d.slower_than_old_state == slower_than_old_state {
                    let old_counter = d.counter;
                    d.counter += delay / 1000 + 1;
                    if old_counter < debounce && d.counter >= debounce {
                        d.counter = 0;
                        d.state = d.staging_state;
                        d.staging_state = new_state;
                    }
                    if d.counter % debounce == 0 {
                        self.refresh(RefreshType::FPS);
                    }
                } else {
                    d.counter = 0;
                    d.staging_state = new_state;
                    d.slower_than_old_state = slower_than_old_state;
                }
            } else {
                user.delay = Some(Delay {
                    state: DelayState::Normal,
                    staging_state: state,
                    delay,
                    counter: 0,
                    slower_than_old_state: None,
                });
            }
        }
    }

    pub fn user_delay_response_elapsed(&mut self, id: i32, elapsed: u128) {
        if let Some(user) = self.users.get_mut(&id) {
            let old = user.response_delayed;
            user.response_delayed = elapsed > 3000;
            if old != user.response_delayed {
                self.refresh(RefreshType::FPS);
            }
        }
    }
    pub fn user_record(&mut self, id: i32, v: bool) {
        if let Some(user) = self.users.get_mut(&id) {
            user.record = v;
        }
    }

    fn user_congested(&mut self, id: i32, congested: bool) {
        if let Some(user) = self.users.get_mut(&id) {
            user.congested = congested;
        }
    }

    fn user_bandwidth(&mut self, id: i32, bandwidth: f32) {
        if let Some(user) = self.users.get_mut(&id) {
            user.bandwidth = bandwidth;
        }
    }

    fn convert_quality(&self, q: i32) -> Quality {
        if q == ImageQuality::Balanced.value() {
            Quality::Balanced
        } else if q == ImageQuality::Low.value() {
            Quality::Low
        } else if q == ImageQuality::Best.value() {
            Quality::Best
        } else {
            let b = ((q >> 8 & 0xFFF) * 2) as f32 / 100.0;
            Quality::Custom(b.clamp(BR_MIN, BR_MAX))
        }
    }

    pub fn refresh(&mut self, typ: RefreshType) {
        log::info!("refresh: {:?}", typ);
        // Update fps
        if typ == RefreshType::All || typ == RefreshType::FPS {
            self.target_fps = self.highest_fps();
            if self.fps > self.target_fps {
                self.fps = self.target_fps;
            }
        }
        if typ == RefreshType::All || typ == RefreshType::SetImageQuality {
            // Update quality
            let mut quality = self.lastest_quality().ratio();
            if quality < BR_MIN || quality > BR_MAX {
                quality = BR_BALANCED;
            }

            // Handle ABR if enabled
            let abr_enabled = self.in_vbr_state();
            if abr_enabled && typ != RefreshType::SetImageQuality {}
            // self.ratio = quality;
        }
    }

    #[inline]
    fn highest_fps(&self) -> u32 {
        let user_fps = |u: &UserData| {
            let mut fps = u.custom_fps.unwrap_or(FPS);
            if let Some(auto_adjust_fps) = u.auto_adjust_fps {
                if fps == 0 || auto_adjust_fps < fps {
                    fps = auto_adjust_fps;
                }
                // delay
                if let Some(delay) = u.delay {
                    fps = match delay.state {
                        DelayState::Normal => fps,
                        DelayState::LowDelay => fps * 3 / 4,
                        DelayState::HighDelay => fps / 2,
                        DelayState::Broken => fps / 4,
                    }
                }
                // delay response
                if u.response_delayed {
                    if fps > MIN_FPS + 2 {
                        fps = MIN_FPS + 2;
                    }
                }
            }
            fps
        };

        let fps = self
            .users
            .iter()
            .map(|(_, u)| user_fps(u))
            .filter(|u| *u >= MIN_FPS)
            .min()
            .unwrap_or(FPS);

        fps.clamp(MIN_FPS, MAX_FPS)
    }
}

// Frame sending record for bandwidth and congestion tracking
#[derive(Debug, Clone, Copy)]
struct SendRecord {
    size: usize,
    timestamp: u128,
}

// Video streaming history for bandwidth estimation and congestion control
#[derive(Debug, Clone)]
pub struct VideoHistory {
    conn_id: i32,
    send_history: Vec<SendRecord>,
    inflight_frames: Vec<SendRecord>,
    first_frame_instant: Option<Instant>,
    rtt: Option<u128>,
    delay: Vec<u128>,
    congested: bool,
    bandwidth_history: Vec<f32>, // bytes per second
    log_timer: Instant,
}

// Basic functionality
impl VideoHistory {
    pub fn new(conn_id: i32) -> Self {
        Self {
            conn_id,
            send_history: Vec::new(),
            inflight_frames: Vec::new(),
            first_frame_instant: None,
            rtt: None,
            delay: Vec::new(),
            congested: false,
            bandwidth_history: Vec::new(),
            log_timer: Instant::now(),
        }
    }

    pub fn on_send(&mut self, size: usize) {
        if size > 50 * 1024 {
            log::info!("on_send: size={}", size);
        }

        let timestamp = self.get_or_init_timestamp();
        let record = SendRecord { size, timestamp };

        self.inflight_frames.push(record);
        rm_first(&mut self.send_history, record, 30);
        self.check_congested();
    }

    pub fn on_receive(&mut self) {
        if self.inflight_frames.is_empty() {
            return;
        }

        let first = self.inflight_frames.remove(0);
        self.update_rtt_and_delay(first);
        self.check_congested();
    }

    fn congested(&mut self) -> bool {
        let inflight = self.inflight_frames.len();
        if self.send_history.len() < 10 {
            return inflight > 1;
        }

        let Some(rtt) = self.rtt else {
            return inflight > 1;
        };
        let rtt = rtt.max(10);

        let ms_per_frame = video_service::VIDEO_QOS.lock().unwrap().spf().as_millis() as f64;
        let base_allowance = (rtt as f64 / ms_per_frame).ceil() as usize;
        let min_allowance = base_allowance + 2;
        if self.log_timer.elapsed().as_secs() >= 1 {
            log::info!("inflight: {:?}, rtt: {:?}, ms_per_frame: {:?}, base_allowance: {:?}, min_allowance: {:?}", inflight, rtt, ms_per_frame, base_allowance, min_allowance);
            self.log_timer = Instant::now();
        }
        inflight > min_allowance
    }
}

// Frame tracking and congestion detection
impl VideoHistory {
    #[inline]
    fn check_congested(&mut self) {
        let old_congested = self.congested;
        self.congested = self.congested();

        if old_congested != self.congested {
            let mut qos = video_service::VIDEO_QOS.lock().unwrap();
            qos.user_congested(self.conn_id, self.congested);
            if self.congested {
                let bandwidth = self.estimate_bandwidth();
                qos.user_bandwidth(self.conn_id, bandwidth);
            }
        }
    }

    #[inline]
    fn get_or_init_timestamp(&mut self) -> u128 {
        match self.first_frame_instant {
            Some(instant) => instant.elapsed().as_millis(),
            None => {
                self.first_frame_instant = Some(Instant::now());
                0
            }
        }
    }

    #[inline]
    fn update_rtt_and_delay(&mut self, frame: SendRecord) {
        let rtt = self.timestamp() as u128 - frame.timestamp;
        self.rtt = match self.rtt {
            Some(old) => Some(std::cmp::min(old, rtt)),
            None => Some(rtt),
        };
        self.delay.push(rtt);
        rm_first(&mut self.delay, rtt, 30);
    }

    fn timestamp(&self) -> i64 {
        // i64 as ms: 2924712086 years
        self.first_frame_instant
            .map(|instant| instant.elapsed().as_millis())
            .unwrap_or(0) as _
    }

    fn estimate_bandwidth(&mut self) -> f32 {
        const WINDOW_SIZE_MS: u128 = 1000; // 1 second window

        if self.send_history.len() < 2 {
            return 0.0;
        }

        let current_time = self.timestamp() as u128;
        let window_start = current_time.saturating_sub(WINDOW_SIZE_MS);

        // Get frames within the time window
        let window_frames: Vec<&SendRecord> = self
            .send_history
            .iter()
            .filter(|record| record.timestamp >= window_start)
            .collect();

        if window_frames.is_empty() {
            return 0.0;
        }

        // Calculate total bytes in window
        let total_bytes: usize = window_frames.iter().map(|record| record.size).sum();

        // Calculate actual time span (considering sub-second windows)
        let time_span = (current_time - window_start) as f32 / 1000.0;
        if time_span <= 0.0 {
            return 0.0;
        }

        // Calculate current bandwidth (bytes per second)
        let current_bw = total_bytes as f32 / time_span;

        // Update bandwidth history
        self.bandwidth_history.push(current_bw);
        rm_first(&mut self.bandwidth_history, current_bw, 30);

        // Calculate exponential moving average
        let alpha = 0.2;
        self.bandwidth_history.iter().fold(0.0, |acc, &x| {
            if acc == 0.0 {
                x
            } else {
                acc * (1.0 - alpha) + x * alpha
            }
        })
    }
}

#[inline]
fn rm_min_max<T: Ord>(v: &mut Vec<T>, push: T, max_len: usize) {
    if v.len() > max_len {
        v.sort();
        v.remove(0);
        v.pop();
    }
    v.push(push);
}

#[inline]
fn rm_first<T>(v: &mut Vec<T>, push: T, max_len: usize) {
    if v.len() > max_len {
        v.remove(0);
    }
    v.push(push);
}
