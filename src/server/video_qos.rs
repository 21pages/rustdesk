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
const MAX_BR_MULTIPLE: f32 = 1.0;

const HISTORY_DELAY_LEN: usize = 6;
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

#[derive(Default, Debug, Clone)]
struct UserDelay {
    response_delayed: bool,
    delay_history: Vec<u32>,
    rtt: Option<u32>,
    fps: Option<u32>,
}

// User session data structure
#[derive(Default, Debug, Clone)]
struct UserData {
    auto_adjust_fps: Option<u32>, // reserve for compatibility
    custom_fps: Option<u32>,
    quality: Option<(i64, Quality)>, // (time, quality)
    delay: UserDelay,
    record: bool,
    support_video_ack: bool,
    congested: bool,
}

struct DisplayData {
    fps: u32,
    send_counter: usize,
}

// Main QoS controller structure
pub struct VideoQoS {
    fps: u32,
    ratio: f32,
    users: HashMap<i32, UserData>,
    displays: HashMap<usize, DisplayData>,
    bitrate_store: u32,
    support_abr: HashMap<usize, bool>,
    adjust_fps_instant: Instant,
    adjust_ratio_instant: Instant,
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
            adjust_fps_instant: Instant::now(),
            adjust_ratio_instant: Instant::now(),
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
        if self.fps > 10 {
            self.fps = 10;
        }
        log::info!("all support video ack: {:?}", self.all_support_video_ack());
    }

    // Clean up user session
    pub fn on_connection_close(&mut self, id: i32) {
        self.users.remove(&id);
        if self.users.is_empty() {
            *self = Default::default();
        }
        if self.users.is_empty() {
            log::info!("all support video ack: {:?}", self.all_support_video_ack());
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
    pub fn remove_display(&mut self, display_idx: usize) {
        self.displays.remove(&display_idx);
    }

    pub fn update_display_fps(&mut self, display_idx: usize, fps: u32, send_counter: usize) {
        if let Some(display) = self.displays.get_mut(&display_idx) {
            display.fps = fps;
            display.send_counter += send_counter;
        } else {
            self.displays
                .insert(display_idx, DisplayData { fps, send_counter });
        }
        if self.adjust_fps_instant.elapsed().as_secs() >= 1 {
            self.adjust_fps_instant = Instant::now();
            if self.all_support_video_ack() {
                self.adjust_fps_based_on_ack();
            } else {
                self.adjust_fps_based_on_delay();
            }
            self.adjust_fps_based_on_ratio();
        }
        let abr_enabled = self.in_vbr_state();
        if abr_enabled {
            if self.adjust_ratio_instant.elapsed().as_secs() >= ADJUST_RATIO_INTERVAL as u64 {
                self.adjust_ratio_instant = Instant::now();
                let dynamic_screen = self
                    .displays
                    .iter()
                    .any(|d| d.1.send_counter > ADJUST_RATIO_INTERVAL * 3);
                println!("dynamic_screen: {:?}", dynamic_screen);
                self.displays.iter_mut().for_each(|d| {
                    println!("send_counter: {:?}", d.1.send_counter);
                    d.1.send_counter = 0;
                });
                self.adjust_ratio(dynamic_screen);
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

    fn adjust_fps_based_on_ratio(&mut self) {
        let target_ratio = self.lastest_quality().ratio();
        let current_ratio = self.ratio;
        let current_fps = self.fps;
        if target_ratio >= BR_BEST {
            if current_ratio < BR_BEST * 0.95 {
                if current_ratio > BR_BALANCED {
                    if current_fps > 22 {
                        self.fps = 22.min(current_fps - 5);
                    }
                } else if current_ratio > BR_SPEED {
                    if current_fps > 18 {
                        self.fps = 18.min(current_fps - 5);
                    }
                } else {
                    if current_fps > 12 {
                        self.fps = 12.min(current_fps - 5);
                    }
                }
            }
        } else if target_ratio >= BR_BALANCED {
            if current_ratio < BR_BALANCED * 0.95 {
                if current_ratio > BR_SPEED {
                    if current_fps > 22 {
                        self.fps = 22.min(current_fps - 5);
                    }
                } else {
                    if current_fps > 18 {
                        self.fps = 18.min(current_fps - 5);
                    }
                }
            }
        } else if target_ratio >= BR_SPEED {
            if current_ratio < BR_SPEED * 0.9 {
                if current_fps > 25 {
                    self.fps = 25.min(current_fps - 5);
                } else if current_fps > 15 {
                    self.fps = 15.min(current_fps - 5);
                }
            }
        } else {
            if current_ratio < target_ratio * 0.9 {
                if current_fps > 15 {
                    self.fps = 15.min(current_fps - 5);
                }
            }
        }
    }

    // Adjust quality ratio based on fps
    fn adjust_ratio(&mut self, dynamic_screen: bool) {
        // Get max average delay from all users
        let mut max_delay = None;
        for (_, user) in self.users.iter() {
            let mut total_delay = 0.0;
            let mut delay_samples = 0;
            if let Some(rtt) = user.delay.rtt {
                let delay_history = user.delay.delay_history.clone();
                let len = delay_history.len();
                if len > 0 {
                    let avg_delay = delay_history.iter().sum::<u32>() as f32 / len as f32;
                    let real_delay = avg_delay - rtt as f32;
                    total_delay += real_delay;
                    delay_samples += 1;
                }
            }
            if delay_samples > 0 {
                let avg_delay = total_delay / delay_samples as f32;
                match max_delay {
                    Some(max) => {
                        if avg_delay > max {
                            max_delay = Some(avg_delay);
                        }
                    }
                    None => max_delay = Some(avg_delay),
                }
            }
        }
        let Some(max_delay) = max_delay else {
            return;
        };
        let target_quality = self.lastest_quality();
        let target_ratio = target_quality.ratio();
        let current_ratio = self.ratio;
        let current_bitrate = self.bitrate();
        // 1Mbps is ok for high resolution during test
        let ratio_1mbps = if current_bitrate > 0 {
            Some((current_ratio * 1000.0 / current_bitrate as f32).max(BR_MIN))
        } else {
            None
        };
        let min = match target_quality {
            Quality::Best => {
                let mut min = BR_BEST / 2.5;
                if let Some(ratio_1mbps) = ratio_1mbps {
                    if min > ratio_1mbps {
                        min = ratio_1mbps;
                    }
                }
                min.max(BR_MIN)
            }
            Quality::Balanced => {
                let mut min = BR_BALANCED / 2.5;
                if let Some(ratio_1mbps) = ratio_1mbps {
                    if min > ratio_1mbps {
                        min = ratio_1mbps;
                    }
                }
                min.max(BR_MIN)
            }
            Quality::Low => BR_MIN,
            Quality::Custom(_) => BR_MIN,
        };
        let max = target_ratio * MAX_BR_MULTIPLE;

        let mut v = current_ratio;
        if max_delay > 500.0 {
            v = current_ratio * 0.85;
        } else if max_delay > 200.0 {
            v = current_ratio * 0.9;
        } else if max_delay > 100.0 {
            v = current_ratio * 0.95;
        } else if max_delay > 50.0 {
            if dynamic_screen {
                v = current_ratio * 1.05;
            }
        } else {
            if dynamic_screen {
                v = current_ratio * 1.1;
            }
        }
        self.ratio = v.clamp(min, max);

        println!(
            "after adjust - ratio: {:.2}, max_delay: {:.1}ms, quality: {:?}, fps: {:?}",
            self.ratio, max_delay, target_quality, self.fps
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
            let len = user.delay.delay_history.len();
            let avg_delay = if len > 0 {
                user.delay.delay_history.iter().sum::<u32>() as f32 / len as f32
            } else {
                delay as f32
            };
            let mut real_delay = avg_delay - user.delay.rtt.unwrap_or_default() as f32;
            real_delay = real_delay.max(10.0);
            let fps = (2000.0 / real_delay).ceil() as u32;
            user.delay.fps = Some(fps.clamp(MIN_FPS, highest_fps));
            println!(
                "avg_delay: {:?}, rtt: {:?}, real_delay: {:?}, fps: {:?}",
                avg_delay, user.delay.rtt, real_delay, user.delay.fps
            );
        }
        self.adjust_fps_based_on_delay();
    }

    pub fn user_delay_response_elapsed(&mut self, id: i32, elapsed: u128) {
        if let Some(user) = self.users.get_mut(&id) {
            user.delay.response_delayed = elapsed > 2000;
            if user.delay.response_delayed {
                rm_first(
                    &mut user.delay.delay_history,
                    elapsed as u32,
                    HISTORY_DELAY_LEN,
                );
            }
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
        if self.users.iter().any(|u| u.1.delay.response_delayed) {
            if fps > MIN_FPS + 1 {
                fps = MIN_FPS + 1;
            }
        }
        self.fps = fps.clamp(MIN_FPS, highest_fps);
    }
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
        for (_, d) in self.displays.iter() {
            if let Some(max) = max_capture_fps {
                max_capture_fps = Some(std::cmp::max(max, d.fps));
            } else {
                max_capture_fps = Some(d.fps);
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
    #[allow(unused)]
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
            println!(
                "inflight: {:?}, rtt: {:?},  base_allowance: {:?}, min_allowance: {:?}",
                max_inflight, rtt, base_allowance, min_allowance
            );
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
