use super::*;
use scrap::codec::Quality;
use std::time::Duration;
use tokio::time::Instant;
use video_service::VIDEO_QOS;
pub const FPS: u32 = 30; // default fps
const FIRST_SECOND_FPS: u32 = 10; // fps in the first second
pub const MIN_FPS: u32 = 1;
pub const MAX_FPS: u32 = 120;
const MIN_AVG_DELAY: u128 = 100; // use average delay as base delay
const USER_DELAY_HISTORY_LEN: usize = 30; // length of UserData.delay_history
const USER_DELAYED_FPS_HISTORY_LEN: usize = 5; // length of UserData.delayed_fps_history
const TRAFFIC_HISTORY_LEN: usize = 5; // length of VideoHistory.congested_traffic, VideoHistory.smooth_traffic
const INFLIGHT_HISTORY_LEN: usize = 5; // length of VideoHistory.inflight
const QOS_HISTORY_FPS_LEN: usize = 10; // length of VideoQoS.history_fps

#[derive(Debug, Clone)]
struct InflightRtt {
    inflight: usize,
    rtt: usize,
    instant: Instant,
}

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
    inflight_rtt: Option<InflightRtt>,  // inflight, round trip time
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
                    if self.fps_debounce % 6 == 0 {
                        if self.delayed_fps_history.len() > 0 {
                            // remove the min delayed fps
                            self.delayed_fps_history.sort();
                            if last_fps + 2 > self.delayed_fps_history[0] {
                                self.delayed_fps_history.remove(0);
                            }
                        }
                        last_fps + 2
                    } else if self.fps_debounce % 3 == 0 {
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
        rm_min_max(
            &mut self.delayed_fps_history,
            fps,
            USER_DELAYED_FPS_HISTORY_LEN,
        );
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
            quality: Default::default(),
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
        // if self.fps >= MIN_FPS && self.fps <= MAX_FPS {
        //     self.fps
        // } else {
        //     FPS
        // }
        FPS
    }

    pub fn congested(&self) -> bool {
        let spf = self.spf().as_millis() as usize;
        self.users
            .iter()
            .filter(|u| u.1.inflight_rtt.is_some())
            .any(|u| {
                if let Some(x) = &u.1.inflight_rtt {
                    log::info!(
                        "inflight={}, rtt={}, spf={}, elapsed:{}",
                        x.inflight,
                        x.rtt,
                        spf,
                        x.instant.elapsed().as_millis()
                    );
                    // return x.instant.elapsed().as_millis() < 300
                    //     && x.inflight > x.rtt.max(100) / spf;
                    return x.instant.elapsed().as_millis() < 300 && x.inflight > 2;
                }
                false
            })
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
            quality = self.delayed_quality(quality);
        }
        self.quality = quality;
    }

    fn delayed_quality(&mut self, user_quality: Quality) -> Quality {
        // avg fps
        let fps = self.fps();
        rm_first(&mut self.history_fps, fps, QOS_HISTORY_FPS_LEN);
        if self.history_fps.len() < QOS_HISTORY_FPS_LEN / 2 {
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
        // println!("avg_fps: {},  quality: {:?}", avg_fps, result);
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
            rm_min_max(&mut user.delay_history, elapsed, USER_DELAY_HISTORY_LEN);
            user.calc_fps();
        }
    }

    pub fn user_inflight_rtt(&mut self, id: i32, inflight: usize, rtt: usize) {
        if let Some(user) = self.users.get_mut(&id) {
            user.inflight_rtt = Some(InflightRtt {
                inflight,
                rtt,
                instant: Instant::now(),
            });
        }
    }

    pub fn user_record(&mut self, id: i32, v: bool) {
        if let Some(user) = self.users.get_mut(&id) {
            user.record = v;
        }
    }

    pub fn on_connection_open(&mut self, id: i32) {
        self.users.insert(
            id,
            UserData {
                ..Default::default()
            },
        );
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
    size: usize,
    timestamp: u128,
}

#[derive(Default, Debug, Clone)]
pub struct VideoHistory {
    conn_id: i32,
    send_history: Vec<SendRecord>,
    first_frame_instant: Option<Instant>,
    last_receive_test_delay_instant: Option<Instant>,
    congested_traffic: Vec<usize>,
    smooth_traffic: Vec<usize>,
    rtt: usize,
}

impl VideoHistory {
    pub fn new(conn_id: i32) -> Self {
        Self {
            conn_id,
            send_history: Vec::new(),
            first_frame_instant: None,
            last_receive_test_delay_instant: None,
            congested_traffic: Vec::new(),
            smooth_traffic: Vec::new(),
            rtt: 0,
        }
    }

    pub fn timestamp(&self) -> i64 {
        // i64 as ms: 2924712086 years
        self.first_frame_instant
            .map(|instant| instant.elapsed().as_millis())
            .unwrap_or(0) as _
    }

    pub fn on_send(&mut self, size: usize) {
        if size > 50 * 1024 {
            log::info!("on_send: size={}", size);
        }
        let timestamp = match self.first_frame_instant {
            Some(instant) => instant.elapsed().as_millis(),
            None => {
                self.first_frame_instant = Some(Instant::now());
                0
            }
        };
        self.send_history.push(SendRecord { size, timestamp });
    }

    pub fn on_receive(&mut self) {
        if self.send_history.len() == 0 {
            return;
        }
        let first = self.send_history.remove(0);
        let rtt = self.timestamp() as u128 - first.timestamp;
        if self.rtt > rtt as usize {
            self.rtt = rtt as usize;
        }
        VIDEO_QOS.lock().unwrap().user_inflight_rtt(
            self.conn_id,
            self.send_history.len(),
            self.rtt,
        );
    }

    pub fn congested(&self) -> bool {
        if self.congested_traffic.len() > 0 {
            let predicted_bandwidth =
                self.congested_traffic.iter().sum::<usize>() / self.congested_traffic.len();
            let sent = self.send_history.iter().map(|s| s.size).sum::<usize>();
            let v = sent > predicted_bandwidth;
            if v {
                log::info!(
                    "congested: sent={}, predicted_bandwidth={}, {:?}",
                    sent,
                    predicted_bandwidth,
                    self.congested_traffic,
                );
            }
            v
        } else {
            false
        }
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
