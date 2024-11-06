use super::*;
use scrap::codec::Quality;
use std::time::Duration;
use tokio::time::Instant;
pub const FPS: u32 = 30; // default fps
const FIRST_SECOND_FPS: u32 = 10; // fps in the first second
pub const MIN_FPS: u32 = 1;
pub const MAX_FPS: u32 = 120;
const MIN_AVG_DELAY: u128 = 100; // use average delay as base delay
const USER_DELAY_HISTORY_LEN: usize = 30; // length of UserData.delay_history
const USER_DELAYED_FPS_HISTORY_LEN: usize = 5; // length of UserData.delayed_fps_history
const QOS_HISTORY_FPS_LEN: usize = 10; // length of VideoQoS.history_fps

#[derive(Default, Debug, Clone)]
struct UserData {
    auto_adjust_fps: Option<u32>,       // reserve for compatibility
    custom_fps: Option<u32>,            // user custom fps
    last_fps: Option<u32>,              // calculated(not real) fps, not change rapidly
    delayed_fps_history: Vec<u32>,      // calculate fps during delay
    fps_debounce: i32,                  // +: increase fps, -: decrease fps
    last_delay: Option<u128>,           // last delay
    delay: Option<u128>,                // current delay
    delay_history: Vec<u128>,           // delay history
    last_recv_instant: Option<Instant>, // instant receive TestDelay
    rx_video_elapsed: Option<u128>,     // last rx_video elapsed
    quality: Option<(i64, Quality)>,    // (time, quality)
    record: bool,                       // recording
}

impl UserData {
    fn calc_fps(&mut self) {
        let mut fps = self.custom_fps.unwrap_or(FPS);
        // auto adjust fps
        if let Some(auto_adjust_fps) = self.auto_adjust_fps {
            if fps == 0 || auto_adjust_fps < fps {
                fps = auto_adjust_fps;
            }
        }
        // delay
        fps = self.delayed_fps(fps);
        if fps < MIN_FPS {
            fps = MIN_FPS;
        }
        if fps > MAX_FPS {
            fps = MAX_FPS;
        }
        self.last_fps = Some(fps);
    }

    // Notice number overflow !!!
    fn delayed_fps(&mut self, max_fps: u32) -> u32 {
        let mut v = match (self.delay, self.last_fps) {
            (Some(delay), Some(last_fps)) => {
                // use average delay as base delay
                let avg_delay = self.get_avg_delay().max(MIN_AVG_DELAY);
                // tolerance at least 100ms
                let delay_tolerance = (avg_delay / 5).max(100);
                if delay > avg_delay + delay_tolerance {
                    // decrease fps
                    if self.fps_debounce > 0 {
                        self.fps_debounce = 0;
                    }
                    self.push_delayed_fps(last_fps);
                    if avg_delay + 1000 < delay {
                        // delay 1000+ms
                        self.fps_debounce = 0;
                        if last_fps > 15 {
                            15
                        } else {
                            last_fps * 5 / 6
                        }
                    } else if avg_delay + 500 < delay {
                        // delay 500~1000ms
                        self.fps_debounce = 0;
                        if last_fps > 25 {
                            25
                        } else if last_fps > 20 {
                            20
                        } else if last_fps > 15 {
                            15
                        } else {
                            last_fps * 6 / 7
                        }
                    } else if avg_delay + 200 < delay && self.fps_debounce < -1 {
                        // delay 200~500ms
                        self.fps_debounce = 0;
                        if last_fps > 25 {
                            25
                        } else if last_fps > 20 {
                            20
                        } else if last_fps > 15 {
                            15
                        } else {
                            last_fps * 7 / 8
                        }
                    } else {
                        // delay 100~200ms
                        if self.fps_debounce > i32::MIN {
                            self.fps_debounce -= 1;
                        }
                        last_fps
                    }
                } else {
                    // increase fps
                    // delay 0~100ms
                    if self.fps_debounce < 0 {
                        self.fps_debounce = 0;
                    }
                    if self.fps_debounce < i32::MAX {
                        self.fps_debounce += 1;
                    }
                    if self.fps_debounce % 10 == 0 {
                        if self.delayed_fps_history.len() > 0 {
                            // remove the min delayed fps
                            self.delayed_fps_history.sort();
                            if last_fps + 2 > self.delayed_fps_history[0] {
                                self.delayed_fps_history.remove(0);
                            }
                        }
                        last_fps + 2
                    } else if self.fps_debounce % 5 == 0 {
                        last_fps + 1
                    } else {
                        last_fps
                    }
                }
            }
            _ => max_fps,
        };

        if self.delayed_fps_history.len() > 0 {
            // not exceed average delayed fps
            let mut sum = 0;
            for i in self.delayed_fps_history.iter() {
                sum += i;
            }
            let avg = sum / self.delayed_fps_history.len() as u32;
            if v > avg {
                v = avg;
            }
        }
        if v > max_fps {
            v = max_fps
        }
        if v < MIN_FPS {
            v = MIN_FPS
        }
        if v > MAX_FPS {
            v = MAX_FPS
        }

        println!(
            "delayed_fps: delay={:?}, avg_delay={:?} fps_array={:?}, debounce={}, last_fps={:?} => fps={}",
            self.delay,
            self.get_avg_delay(),
            self.delayed_fps_history,
            self.fps_debounce,
            self.last_fps,
            v,
        );
        v
    }

    fn push_delayed_fps(&mut self, fps: u32) {
        if self.delayed_fps_history.contains(&fps) {
            // avoid all is 1
            return;
        }
        if self.delayed_fps_history.len() > USER_DELAYED_FPS_HISTORY_LEN {
            // remove the max and the min values
            self.delayed_fps_history.sort();
            self.delayed_fps_history.remove(0);
            self.delayed_fps_history.pop();
        }
        self.delayed_fps_history.push(fps);
    }

    fn get_fps(&self) -> u32 {
        match (self.last_fps, self.last_recv_instant, self.rx_video_elapsed) {
            (Some(fps), Some(last_recv_instant), Some(rx_video_elapsed)) => {
                // TestDelay or video channel delayed
                let elapsed = last_recv_instant.elapsed().as_millis();
                if elapsed > crate::server::TEST_DELAY_TIMEOUT.as_millis() + 1000
                    || rx_video_elapsed > 200
                {
                    MIN_FPS
                } else {
                    fps
                }
            }
            _ => FIRST_SECOND_FPS, // wait TestDelay
        }
    }

    #[inline]
    fn get_avg_delay(&self) -> u128 {
        let len = self.delay_history.len();
        if len > 0 {
            self.delay_history.iter().sum::<u128>() / len as u128
        } else {
            self.last_delay.unwrap_or(MIN_AVG_DELAY)
        }
    }
}

pub struct VideoQoS {
    fps: u32,
    quality: Quality,
    delayed_quality: Option<Quality>,
    users: HashMap<i32, UserData>,
    bitrate_store: u32,
    support_abr: HashMap<usize, bool>,
    history_fps: Vec<u32>,
}

impl Default for VideoQoS {
    fn default() -> Self {
        VideoQoS {
            fps: FPS,
            quality: Quality::Bitrate(crate::START_BITRATE),
            delayed_quality: Default::default(),
            users: Default::default(),
            bitrate_store: 0,
            support_abr: Default::default(),
            history_fps: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RefreshType {
    SetImageQuality,
    Timer,
}

impl VideoQoS {
    pub fn spf(&self) -> Duration {
        Duration::from_secs_f32(1. / (self.fps() as f32))
    }

    pub fn fps(&self) -> u32 {
        if self.fps >= MIN_FPS && self.fps <= MAX_FPS {
            self.fps
        } else {
            FPS
        }
        // FPS
    }

    pub fn change_bitrate_directly(&mut self, bitrate: u32) {
        self.bitrate_store = bitrate;
        self.quality = Quality::Bitrate(bitrate);
    }

    pub fn store_bitrate(&mut self, bitrate: u32) {
        self.bitrate_store = bitrate;
    }

    pub fn bitrate(&self) -> u32 {
        self.bitrate_store
    }

    pub fn quality(&self) -> Quality {
        self.quality
    }

    pub fn record(&self) -> bool {
        self.users.iter().any(|u| u.1.record)
    }

    pub fn set_support_abr(&mut self, display_idx: usize, support: bool) {
        self.support_abr.insert(display_idx, support);
    }

    pub fn in_vbr_state(&self) -> bool {
        Config::get_option("enable-abr") != "N" && self.support_abr.iter().all(|e| *e.1)
    }

    pub fn refresh(&mut self, typ: Option<RefreshType>) {
        // fps
        let mut fps = self
            .users
            .iter_mut()
            .map(|(_, u)| u.get_fps())
            .filter(|u| *u >= MIN_FPS)
            .min()
            .unwrap_or(FPS);
        if fps > MAX_FPS {
            fps = MAX_FPS;
        }
        self.fps = fps;
        return;

        // quality
        // latest image quality
        let latest_quality = self
            .users
            .iter()
            .map(|(_, u)| u.quality)
            .filter(|q| *q != None)
            .max_by(|a, b| a.unwrap_or_default().0.cmp(&b.unwrap_or_default().0))
            .unwrap_or_default()
            .unwrap_or_default()
            .1;
        let mut quality = latest_quality;

        // network delay
        let abr_enabled = self.in_vbr_state();
        if abr_enabled && typ != Some(RefreshType::SetImageQuality) {
            // quality = self.delayed_quality(quality);
        }
        self.quality = quality;
    }

    fn delayed_quality(&mut self, user_quality: Quality) -> Quality {
        // avg fps
        let max_history_fps_len = QOS_HISTORY_FPS_LEN;
        if self.history_fps.len() > max_history_fps_len {
            self.history_fps.remove(0);
        }
        self.history_fps.push(self.fps());
        if self.history_fps.len() < max_history_fps_len / 2 {
            return user_quality;
        }
        let avg_fps = self.history_fps.iter().sum::<u32>() / self.history_fps.len() as u32;

        // fps too low
        let result = if avg_fps < 10 {
            // User quality will keep unchanged unless new connection, new disconnection or new image quality setting.
            // Each user quality has a unique corresponding delayed quality.
            let delayed_quality = match user_quality {
                Quality::Best => Quality::Balanced,
                Quality::Balanced => Quality::Low,
                Quality::Low => Quality::Low,
                Quality::Custom(b) => Quality::Custom((b / 2).max(20)),
                Quality::Bitrate(b) => Quality::Bitrate(b / 2),
            };
            self.delayed_quality = Some(delayed_quality);
            delayed_quality
        } else if let Some(delayed_quality) = self.delayed_quality {
            // keep delayed quality if fps < 20
            if self.quality == delayed_quality && avg_fps < 20 {
                delayed_quality
            } else {
                user_quality
            }
        } else {
            user_quality
        };
        if Some(result) != self.delayed_quality {
            self.delayed_quality = None;
        }
        println!("avg_fps: {},  quality: {:?}", avg_fps, result);
        result
    }

    pub fn user_custom_fps(&mut self, id: i32, fps: u32) {
        if fps < MIN_FPS {
            return;
        }
        if let Some(user) = self.users.get_mut(&id) {
            user.custom_fps = Some(fps);
        } else {
            self.users.insert(
                id,
                UserData {
                    custom_fps: Some(fps),
                    ..Default::default()
                },
            );
        }
        self.refresh(None);
    }

    pub fn user_auto_adjust_fps(&mut self, id: i32, fps: u32) {
        if let Some(user) = self.users.get_mut(&id) {
            user.auto_adjust_fps = Some(fps);
        } else {
            self.users.insert(
                id,
                UserData {
                    auto_adjust_fps: Some(fps),
                    ..Default::default()
                },
            );
        }
        self.refresh(None);
    }

    pub fn user_image_quality(&mut self, id: i32, image_quality: i32) {
        // https://github.com/rustdesk/rustdesk/blob/d716e2b40c38737f1aa3f16de0dec67394a6ac68/src/server/video_service.rs#L493
        let convert_quality = |q: i32| {
            if q == ImageQuality::Balanced.value() {
                Quality::Balanced
            } else if q == ImageQuality::Low.value() {
                Quality::Low
            } else if q == ImageQuality::Best.value() {
                Quality::Best
            } else {
                let mut b = (q >> 8 & 0xFFF) * 2;
                b = std::cmp::max(b, 20);
                b = std::cmp::min(b, 8000);
                Quality::Custom(b as u32)
            }
        };

        let quality = Some((hbb_common::get_time(), convert_quality(image_quality)));
        if let Some(user) = self.users.get_mut(&id) {
            user.quality = quality;
        } else {
            self.users.insert(
                id,
                UserData {
                    quality,
                    ..Default::default()
                },
            );
        }
        self.refresh(Some(RefreshType::SetImageQuality));
    }

    pub fn user_test_delay(
        &mut self,
        id: i32,
        send: Option<Instant>,
        recv: Option<Instant>,
        last_rx_video_elapsed: Option<u128>,
    ) {
        let elapsed = match (send, recv) {
            (Some(send), Some(recv)) if recv > send => recv.checked_duration_since(send),
            (Some(send), None) => Some(send.elapsed()),
            _ => {
                return;
            }
        };
        let Some(elapsed) = elapsed else {
            return;
        };
        let elapsed = elapsed.as_millis();
        if let Some(user) = self.users.get_mut(&id) {
            user.last_delay = user.delay;
            user.delay = Some(elapsed);
            if let Some(recv) = recv {
                user.last_recv_instant = Some(recv);
            }
            user.rx_video_elapsed = last_rx_video_elapsed;
            if user.delay_history.len() > USER_DELAY_HISTORY_LEN {
                user.delay_history.sort();
                user.delay_history.remove(0);
                user.delay_history.pop();
            }
            user.delay_history.push(elapsed);
            user.calc_fps();
        }
    }

    pub fn user_record(&mut self, id: i32, v: bool) {
        if let Some(user) = self.users.get_mut(&id) {
            user.record = v;
        }
    }

    pub fn on_connection_open(&mut self, id: i32) {
        self.users.insert(id, Default::default());
        self.refresh(None);
    }
    pub fn on_connection_close(&mut self, id: i32) {
        self.users.remove(&id);
        if self.users.len() == 0 {
            *self = Default::default();
        }
        self.refresh(None);
    }
}

#[derive(Debug, Clone, Copy)]
struct SendRecord {
    id: u32,
    size: u64,
    timestamp: u32,
}

struct RecvRecord {
    id: u32,
    rtt: u32,
}

pub struct VideoHistory {
    send_history: Vec<SendRecord>,
    recv_history: Vec<RecvRecord>,
}

impl VideoHistory {
    pub fn new() -> Self {
        VideoHistory {
            send_history: Default::default(),
            recv_history: Default::default(),
        }
    }

    pub fn on_send(&mut self, id: u32, size: u64, timestamp: u32) {
        self.send_history.push(SendRecord {
            id,
            size,
            timestamp,
        });
    }

    pub fn on_receive(&mut self, id: u32, current_timestamp: u32) -> Option<u32> {
        let send = self.send_history.iter().find(|s| s.id == id).map(|s| *s);
        log::info!(
            "send: {:?}, id: {id}, current_timestamp: {current_timestamp}",
            send
        );
        if id == 0 {
            self.send_history.clear();
        } else {
            self.send_history.retain(|s| s.id > id);
        }
        if let Some(send) = send {
            if current_timestamp > send.timestamp {
                let rtt = current_timestamp - send.timestamp;
                self.recv_history.push(RecvRecord { id, rtt });
                if self.recv_history.len() > 10 {
                    self.recv_history.remove(0);
                }
                Some(rtt)
            } else {
                None
            }
        } else {
            None
        }
    }
}
