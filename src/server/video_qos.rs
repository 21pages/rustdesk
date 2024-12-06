use super::*;
use std::time::{Duration, Instant};

// Constants
pub const FPS: u32 = 30;
pub const MIN_FPS: u32 = 1;
pub const MAX_FPS: u32 = 120;

pub const USE_VIDEO_ACK: bool = true; // false;

// Bitrate ratio constants for different quality levels
const BR_MAX: f32 = 40.0;
const BR_MIN: f32 = 0.2;
const BR_BEST: f32 = 1.5;
const BR_BALANCED: f32 = 1.0;
const BR_SPEED: f32 = 0.67;

const FPS_HISTORY_LEN: usize = 6;
const HISTORY_DELAY_LEN: usize = 6;
const SEND_RECORD_LEN: usize = 30;
const ADJUST_RATIO_INTERVAL: usize = 3;

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
    pub fn ratio(&self) -> f32 {
        match self {
            Quality::Best => BR_BEST,
            Quality::Balanced => BR_BALANCED,
            Quality::Low => BR_SPEED,
            Quality::Custom(v) => *v,
        }
    }
}

// User session data structure
#[derive(Default, Debug, Clone)]
struct UserData {
    auto_adjust_fps: Option<u32>, // reserve for compatibility
    custom_fps: Option<u32>,
    quality: Option<(i64, Quality)>, // (time, quality)
    delay: UserDelay,
    response_delayed: bool,
    record: bool,
    support_video_ack: bool,
    congested: bool,
}

#[derive(Default, Debug, Clone)]
struct DisplayData {
    fps_history: Vec<u32>,
}

// Main QoS controller structure
pub struct VideoQoS {
    fps: u32,
    ratio: f32,
    users: HashMap<i32, UserData>,
    bitrate_store: u32,
    support_abr: HashMap<usize, bool>,
    displays: HashMap<usize, DisplayData>,
    fps_instant: Instant,
    ratio_instant: Instant,
}

impl Default for VideoQoS {
    fn default() -> Self {
        VideoQoS {
            fps: FPS,
            ratio: BR_BALANCED,
            users: Default::default(),
            bitrate_store: 0,
            support_abr: Default::default(),
            displays: Default::default(),
            fps_instant: Instant::now(),
            ratio_instant: Instant::now(),
        }
    }
}

// Basic functionality
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

    // Store bitrate for later use
    pub fn store_bitrate(&mut self, bitrate: u32) {
        self.bitrate_store = bitrate;
    }

    // Get stored bitrate
    pub fn bitrate(&self) -> u32 {
        self.bitrate_store
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
}

// User session management
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
    }

    // Clean up user session
    pub fn on_connection_close(&mut self, id: i32) {
        self.users.remove(&id);
        if self.users.is_empty() {
            *self = Default::default();
        }
    }

    pub fn user_custom_fps(&mut self, id: i32, fps: u32) {
        if fps < MIN_FPS {
            return;
        }
        if let Some(user) = self.users.get_mut(&id) {
            user.custom_fps = Some(fps);
        }
    }

    pub fn user_auto_adjust_fps(&mut self, id: i32, fps: u32) {
        if let Some(user) = self.users.get_mut(&id) {
            user.auto_adjust_fps = Some(fps);
        }
    }

    pub fn user_image_quality(&mut self, id: i32, image_quality: i32) {
        let convert_quality = |q: i32| -> Quality {
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
        };

        let quality = Some((hbb_common::get_time(), convert_quality(image_quality)));
        if let Some(user) = self.users.get_mut(&id) {
            log::info!("user.quality: {:?}", user.quality);
            user.quality = quality;
        }
    }

    pub fn user_record(&mut self, id: i32, v: bool) {
        if let Some(user) = self.users.get_mut(&id) {
            user.record = v;
        }
    }
}

// Common adjust functions
impl VideoQoS {
    pub fn new_display(&mut self, display_idx: usize) {
        self.displays.insert(display_idx, DisplayData::default());
    }

    pub fn remove_display(&mut self, display_idx: usize) {
        self.displays.remove(&display_idx);
    }

    pub fn update_display_fps(&mut self, display_idx: usize, fps: u32) {
        log::info!("update_display_fps: {:?}", fps);
        if let Some(display) = self.displays.get_mut(&display_idx) {
            rm_first(&mut display.fps_history, fps, FPS_HISTORY_LEN);
        }
        if self.fps_instant.elapsed().as_secs() >= 1 {
            self.fps_instant = Instant::now();
            if self.all_support_video_ack() {
                self.adjust_fps_based_on_ack();
            } else {
                self.adjust_fps_based_on_delay();
            }
        }
        let abr_enabled = self.in_vbr_state();
        if abr_enabled {
            if self.ratio_instant.elapsed().as_secs() >= ADJUST_RATIO_INTERVAL as u64 {
                self.ratio_instant = Instant::now();
                self.adjust_ratio();
            }
        } else {
            self.ratio = self.lastest_quality().ratio();
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

    // Adjust quality ratio based on fps
    fn adjust_ratio(&mut self) {
        let displays = self.displays.clone();
        let mut num = 0;
        let mut den = 0;
        for (_, display) in displays.iter() {
            for fps in display.fps_history.iter() {
                num += *fps;
                den += 1;
            }
        }
        if den == 0 {
            return;
        }
        let avg_fps = num as f32 / den as f32;
        let target_quality = self.lastest_quality();
        let target_ratio = target_quality.ratio();
        let (min, max) = if self.all_support_video_ack() {
            (BR_MIN, (target_ratio * 1.0).min(BR_MAX))
        } else {
            (BR_MIN, target_ratio)
        };
        log::info!(
            "min: {:?}, max: {:?}, all_support_video_ack: {:?}, avg_fps: {:?}, num: {:?}, den: {:?}",
            min,
            max,
            self.all_support_video_ack(),
            avg_fps,
            num,
            den
        );
        let highest_fps = self.highest_fps();
        let fps_ratio = avg_fps / highest_fps as f32;
        let current_ratio = self.ratio;
        let mut v = self.ratio;

        // Basic guarantees for any quality mode
        if highest_fps > 20 && avg_fps < 10.0 {
            // When highest_fps > 20, ensure fps not lower than 10
            v = current_ratio * 0.9; // Aggressive quality reduction
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
                Quality::Balanced | Quality::Custom(_) => {
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
            }
        }

        // // Apply minimum ratio guarantees based on FPS
        // if avg_fps > 15.0 {
        //     v = v.max(BR_BALANCED);
        // } else if avg_fps > 10.0 {
        //     v = v.max(BR_SPEED);
        // }

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
}

// Adjust based on TestDelay
impl VideoQoS {
    pub fn user_network_delay(&mut self, id: i32, delay: u32) {
        let highest_fps = self.highest_fps();
        if let Some(user) = self.users.get_mut(&id) {
            let delay = delay.max(10);
            rm_first(&mut user.delay.delay_history, delay, HISTORY_DELAY_LEN);
            match user.delay.rtt {
                Some(rtt) => {
                    if rtt > delay {
                        user.delay.rtt = Some(delay);
                    }
                }
                None => user.delay.rtt = Some(delay),
            }
            let mut avg_delay = user.delay.delay_history.iter().sum::<u32>() as f32
                / user.delay.delay_history.len() as f32;
            println!("avg_delay: {:?}, rtt: {:?}", avg_delay, user.delay.rtt);
            avg_delay = avg_delay.min(200.0);
            let mut real_delay = avg_delay - user.delay.rtt.unwrap_or_default() as f32;
            real_delay = real_delay.max(10.0);
            let fps = (2000.0 / real_delay).ceil() as u32;
            user.delay.fps = Some(fps.clamp(MIN_FPS, highest_fps));
        }
        self.adjust_fps_based_on_delay();
    }

    pub fn user_delay_response_elapsed(&mut self, id: i32, elapsed: u128) {
        if let Some(user) = self.users.get_mut(&id) {
            user.response_delayed = elapsed > 2000;
        }
    }

    fn adjust_fps_based_on_delay(&mut self) {
        let highest_fps = self.highest_fps();
        let mut fps = self
            .users
            .iter()
            .map(|u| u.1.delay.fps)
            .filter(|f| *f != None)
            .min()
            .flatten()
            .unwrap_or(FPS);
        if self.users.iter().any(|u| u.1.response_delayed) {
            if fps > MIN_FPS + 2 {
                fps = MIN_FPS + 2;
            }
        }
        self.fps = fps.clamp(MIN_FPS, highest_fps);
    }
}

#[derive(Default, Debug, Clone)]
struct UserDelay {
    delay_history: Vec<u32>,
    rtt: Option<u32>,
    fps: Option<u32>,
}

// Adjust based on video ack
impl VideoQoS {
    fn all_support_video_ack(&self) -> bool {
        self.users.iter().all(|u| u.1.support_video_ack)
    }

    fn user_congested(&mut self, id: i32, congested: bool) {
        if let Some(user) = self.users.get_mut(&id) {
            user.congested = congested;
        }
    }

    // Main congestion control function
    pub fn congested(&mut self) -> bool {
        if !self.all_support_video_ack() {
            return false;
        }
        self.users.iter().any(|u| u.1.congested)
    }

    // Calculate FPS based on congestion status
    #[inline]
    fn adjust_fps_based_on_ack(&mut self) {
        let highest_fps = self.highest_fps();
        let mut max_capture_fps = None;
        for (_, display) in self.displays.iter() {
            if let Some(fps) = display.fps_history.last() {
                if let Some(max) = max_capture_fps {
                    max_capture_fps = Some(std::cmp::max(max, *fps));
                } else {
                    max_capture_fps = Some(*fps);
                }
            }
        }
        if let Some(fps) = max_capture_fps {
            self.fps = (fps + 2).clamp(MIN_FPS, highest_fps);
        }
    }
}

// Frame sending record for bandwidth and congestion tracking
#[derive(Debug, Clone, Copy)]
struct SendRecord {
    display: usize,
    size: usize,
    timestamp: u128,
}

// Video streaming history for bandwidth estimation and congestion control
#[derive(Debug, Clone)]
pub struct VideoHistory {
    conn_id: i32,
    inflight_frames: Vec<SendRecord>,
    first_frame_instant: Option<Instant>,
    rtt: Option<u128>,
    delay: Vec<u128>,
    congested: bool,
    second_timer: Instant,
    send_record: Vec<SendRecord>,
}

// Public functionality
impl VideoHistory {
    pub fn new(conn_id: i32) -> Self {
        Self {
            conn_id,
            inflight_frames: Vec::new(),
            first_frame_instant: None,
            rtt: None,
            delay: Vec::new(),
            congested: false,
            second_timer: Instant::now(),
            send_record: Vec::new(),
        }
    }

    pub fn on_send(&mut self, value: &Arc<Message>) {
        let Some(message::Union::VideoFrame(ref vf)) = value.union else {
            return;
        };

        let size = value.compute_size() as usize;
        let display = vf.display as usize;
        if size > 50 * 1024 {
            println!("on_send: size={}", size);
        }
        let timestamp = self.get_or_init_timestamp();
        let record = SendRecord {
            display,
            size,
            timestamp,
        };
        self.inflight_frames.push(record);
        rm_first(&mut self.send_record, record, SEND_RECORD_LEN);
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
}

// Private functions
impl VideoHistory {
    #[inline]
    fn check_congested(&mut self) {
        let old_congested = self.congested;
        self.congested = self.congested();

        if old_congested != self.congested {
            let mut qos = video_service::VIDEO_QOS.lock().unwrap();
            qos.user_congested(self.conn_id, self.congested);
        }
    }
    fn congested(&mut self) -> bool {
        let inflight_frames = self.inflight_frames.clone();
        let displays = inflight_frames
            .iter()
            .map(|f| f.display)
            .collect::<std::collections::HashSet<_>>();
        let mut max_inflight = 0;
        for display in displays {
            let inflight = inflight_frames
                .iter()
                .filter(|f| f.display == display)
                .count();
            if inflight > max_inflight {
                max_inflight = inflight;
            }
        }
        let Some(rtt) = self.rtt else {
            return max_inflight > 1;
        };
        let rtt = rtt.max(10);
        let ms_per_frame = 1000. / video_service::VIDEO_QOS.lock().unwrap().highest_fps() as f64;
        let base_allowance = (rtt as f64 / ms_per_frame).ceil() as usize;
        let min_allowance = base_allowance + 2;
        if self.second_timer.elapsed().as_secs() >= 1 {
            println!("inflight: {:?}, rtt: {:?}, ms_per_frame: {:?}, base_allowance: {:?}, min_allowance: {:?}", max_inflight, rtt, ms_per_frame, base_allowance, min_allowance);
            self.second_timer = Instant::now();
        }
        max_inflight > min_allowance
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
}

#[inline]
fn rm_first<T>(v: &mut Vec<T>, push: T, max_len: usize) {
    if v.len() > max_len {
        v.remove(0);
    }
    v.push(push);
}
