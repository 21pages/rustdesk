use super::*;
use scrap::codec::{Quality, QualityValue};
use std::time::Duration;
pub const FPS: u32 = 30;
pub const MIN_FPS: u32 = 1;
pub const MAX_FPS: u32 = 120;
trait Percent {
    fn as_percent(&self) -> u32;
}

impl Percent for ImageQuality {
    fn as_percent(&self) -> u32 {
        match self {
            ImageQuality::NotSet => 0,
            ImageQuality::Low => 50,
            ImageQuality::Balanced => 66,
            ImageQuality::Best => 100,
        }
    }
}

#[derive(Default, Debug, Copy, Clone)]
struct Delay {
    state: DelayState,
    delay: u32,
    counter: u32,
    slower_than_old_state: Option<bool>,
}

#[derive(Default, Debug, Copy, Clone)]
struct UserData {
    full_speed_fps: Option<u32>,
    custom_fps: Option<u32>,
    quality: Option<(i64, Quality)>, // (time, quality)
    delay: Option<Delay>,
}

pub struct VideoQoS {
    width: u32,
    height: u32,
    fps: u32,
    quality: Quality,
    users: HashMap<i32, UserData>,
    bitrate_store: u32,
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

impl Default for VideoQoS {
    fn default() -> Self {
        VideoQoS {
            fps: FPS,
            width: 0,
            height: 0,
            quality: Default::default(),
            users: Default::default(),
            bitrate_store: 0,
        }
    }
}

impl VideoQoS {
    pub fn set_size(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
    }

    pub fn spf(&self) -> Duration {
        Duration::from_secs_f32(1. / (self.fps as f32))
    }

    pub fn fps(&self) -> u32 {
        self.fps
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

    pub fn abr_enabled() -> bool {
        "N" != Config::get_option("enable-abr")
    }

    pub fn refresh(&mut self) {
        // fps
        let user_fps = |u: &UserData| {
            // full_speed_fps
            let mut fps = u.full_speed_fps.unwrap_or_default() * 9 / 10;
            // custom_fps
            if let Some(custom_fps) = u.custom_fps {
                if fps == 0 || custom_fps < fps {
                    fps = custom_fps;
                }
            }
            // delay
            if let Some(delay) = u.delay {
                fps = match delay.state {
                    DelayState::Normal => fps,
                    DelayState::LowDelay => fps,
                    DelayState::HighDelay => fps / 2,
                    DelayState::Broken => fps / 4,
                }
            }
            return fps;
        };
        let mut fps = self
            .users
            .iter()
            .map(|(_, u)| user_fps(u))
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
        if Self::abr_enabled() {
            // max delay
            let delay = self
                .users
                .iter()
                .map(|u| u.1.delay)
                .filter(|d| d.is_some())
                .max_by(|a, b| {
                    (a.unwrap_or_default().state as u32).cmp(&(b.unwrap_or_default().state as u32))
                });
            let delay = delay.unwrap_or_default().unwrap_or_default().state;
            if delay != DelayState::Normal {
                match self.quality {
                    Quality::Best => {
                        quality = Quality::Balanced;
                    }
                    Quality::Balanced => {
                        quality = Quality::Low;
                    }
                    Quality::Low => {
                        quality = Quality::Low;
                    }
                    Quality::Custom(v) => match delay {
                        DelayState::LowDelay => {
                            quality = Quality::Custom(QualityValue {
                                q_min: v.q_min,
                                b: std::cmp::min(50, v.b),
                            });
                        }
                        DelayState::HighDelay => {
                            quality = Quality::Custom(QualityValue {
                                q_min: v.q_min,
                                b: std::cmp::min(25, v.b),
                            });
                        }
                        DelayState::Broken => {
                            quality = Quality::Custom(QualityValue {
                                q_min: v.q_min,
                                b: std::cmp::min(10, v.b),
                            });
                        }
                        DelayState::Normal => {}
                    },
                }
            }
        }
        self.quality = quality;
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
        self.refresh();
    }

    pub fn user_full_speed_fps(&mut self, id: i32, full_speed_fps: u32) {
        if let Some(user) = self.users.get_mut(&id) {
            user.full_speed_fps = Some(full_speed_fps);
        } else {
            self.users.insert(
                id,
                UserData {
                    full_speed_fps: Some(full_speed_fps),
                    ..Default::default()
                },
            );
        }
        self.refresh();
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
                let b = (q >> 8 & 0xFF) * 2;
                let q = q & 0xFF;
                Quality::Custom(QualityValue {
                    q_min: q as _,
                    b: b as _,
                })
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
        self.refresh();
    }

    pub fn user_network_delay(&mut self, id: i32, delay: u32) {
        let mut refresh = true;
        let state = DelayState::from_delay(delay);
        if let Some(user) = self.users.get_mut(&id) {
            if let Some(d) = &mut user.delay {
                d.delay = (delay + d.delay) / 2;
                let new_state = DelayState::from_delay(d.delay);
                let slower_than_old_state = new_state as i32 - d.state as i32;
                let slower_than_old_state = if slower_than_old_state > 0 {
                    Some(true)
                } else if slower_than_old_state < 0 {
                    Some(false)
                } else {
                    None
                };
                if d.slower_than_old_state == slower_than_old_state {
                    d.counter += 1;
                    let debounce = 3;
                    refresh = d.counter == debounce;
                    if refresh {
                        d.state = new_state;
                    }
                } else {
                    d.counter = 0;
                    d.state = new_state;
                    d.slower_than_old_state = slower_than_old_state;
                    refresh = false;
                }
            } else {
                user.delay = Some(Delay {
                    state,
                    delay,
                    counter: 0,
                    slower_than_old_state: None,
                });
            }
        } else {
            self.users.insert(
                id,
                UserData {
                    delay: Some(Delay {
                        state,
                        delay,
                        counter: 0,
                        slower_than_old_state: None,
                    }),
                    ..Default::default()
                },
            );
        }
        if refresh {
            self.refresh();
        }
    }

    pub fn on_connection_close(&mut self, id: i32) {
        self.users.remove(&id);
        self.refresh();
    }
}
