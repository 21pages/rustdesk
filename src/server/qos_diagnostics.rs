use hbb_common::{
    get_time, log,
    message_proto::{message, video_frame, Message, VideoFrame},
};
use std::{
    collections::{BTreeMap, HashSet},
    sync::{mpsc, Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

const SLOW: Duration = Duration::from_millis(100);

#[derive(Default)]
pub(super) struct Monitor {
    shared: Option<Arc<Shared>>,
    // Disconnecting this channel also stops the observer on early returns.
    _stop: Option<mpsc::Sender<()>>,
}

struct Shared {
    scope: String,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    active: Option<Active>,
    stats: BTreeMap<(&'static str, &'static str), Stats>,
    timers: BTreeMap<&'static str, Instant>,
    waiting_for: Vec<i32>,
    target_fps: f64,
    ack_required: bool,
    encoded_frames: usize,
    encoded_bytes: usize,
}

#[derive(Clone, Copy)]
struct Active {
    stage: &'static str,
    kind: &'static str,
    display: i32,
    start: Instant,
    start_t: i64,
    queued: Option<Instant>,
}

#[derive(Default)]
struct Stats {
    count: u64,
    ok: u64,
    errors: u64,
    total: Duration,
    max: Duration,
    queue_max: Duration,
}

struct Snapshot {
    active: Option<Active>,
    stats: BTreeMap<(&'static str, &'static str), Stats>,
    timer_overdue: Vec<(&'static str, Duration)>,
    waiting_for: Vec<i32>,
    target_fps: f64,
    ack_required: bool,
    encoded_frames: usize,
    encoded_bytes: usize,
}

pub(super) struct Span {
    shared: Option<Arc<Shared>>,
    active: Option<Active>,
    previous: Option<Active>,
}

impl Monitor {
    pub fn new(scope: impl FnOnce() -> String) -> Self {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        if !*ENABLED.get_or_init(|| std::env::var("RUSTDESK_QOS_DIAGNOSTICS").as_deref() == Ok("1"))
        {
            return Self::default();
        }
        Self::start(scope(), Duration::from_secs(1), Snapshot::log)
    }

    fn start(
        scope: String,
        period: Duration,
        report: impl Fn(&str, Snapshot, Duration) + Send + 'static,
    ) -> Self {
        let shared = Arc::new(Shared {
            scope,
            state: Mutex::new(State::default()),
        });
        let observer = Arc::clone(&shared);
        let (tx, rx) = mpsc::channel();
        // A Tokio task cannot observe a synchronously blocked runtime worker.
        let thread = std::thread::Builder::new()
            .name("qos-diagnostics".into())
            .spawn(move || {
                let mut last = Instant::now();
                while let Err(mpsc::RecvTimeoutError::Timeout) = rx.recv_timeout(period) {
                    let now = Instant::now();
                    let snapshot = observer.snapshot(now);
                    report(&observer.scope, snapshot, now.duration_since(last));
                    last = now;
                }
            });
        if let Err(err) = thread {
            log::error!("Failed to start QoS diagnostics: {err}");
            return Self::default();
        }
        log::info!(
            "qos_diag t={} scope={} event=start",
            get_time(),
            shared.scope
        );
        Self {
            shared: Some(shared),
            _stop: Some(tx),
        }
    }

    pub fn enter(&self, stage: &'static str) -> Span {
        self.begin(stage, "none", -1, None)
    }

    pub fn is_enabled(&self) -> bool {
        self.shared.is_some()
    }

    pub fn socket_write(&self, display: Option<usize>, queued: Instant) -> Span {
        self.begin(
            "socket_write",
            if display.is_some() { "video" } else { "wire" },
            display.map_or(-1, |display| display as i32),
            Some(queued),
        )
    }

    pub fn message(&self, stage: &'static str, msg: &Message, queued: Option<Instant>) -> Span {
        let (kind, display) = match &msg.union {
            Some(message::Union::VideoFrame(vf)) => ("video", vf.display),
            Some(message::Union::TestDelay(_)) => ("probe", -1),
            Some(message::Union::AudioFrame(_)) => ("audio", -1),
            Some(message::Union::MultiClipboards(_)) => ("clipboard", -1),
            _ => ("other", -1),
        };
        self.begin(stage, kind, display, queued)
    }

    fn begin(
        &self,
        stage: &'static str,
        kind: &'static str,
        display: i32,
        queued: Option<Instant>,
    ) -> Span {
        let mut span = Span {
            shared: None,
            active: None,
            previous: None,
        };
        if let Some(shared) = &self.shared {
            let active = Active {
                stage,
                kind,
                display,
                start: Instant::now(),
                start_t: get_time(),
                queued,
            };
            span.previous = shared.state.lock().unwrap().active.replace(active);
            span.active = Some(active);
            span.shared = Some(Arc::clone(shared));
            // Probes are infrequent. Per-frame begin/end logs are deliberately avoided.
            if kind == "probe" {
                log::info!(
                    "qos_diag t={} scope={} event=begin stage={} kind=probe queue_ms={}",
                    active.start_t,
                    shared.scope,
                    stage,
                    active.queue_age(active.start).as_millis()
                );
            }
        }
        span
    }

    pub fn arm_timer(&self, name: &'static str, deadline: Instant) {
        if let Some(shared) = &self.shared {
            shared.state.lock().unwrap().timers.insert(name, deadline);
        }
    }

    pub fn timer(&self, name: &'static str, scheduled: Instant, period: Duration) {
        if let Some(shared) = &self.shared {
            let now = Instant::now();
            self.arm_timer(name, now + period);
            let late = now.saturating_duration_since(scheduled);
            if late >= SLOW {
                log::info!(
                    "qos_diag t={} scope={} event=timer name={} late_ms={}",
                    get_time(),
                    shared.scope,
                    name,
                    late.as_millis()
                );
            }
        }
    }

    pub fn video_settings(&self, spf: Duration) {
        if let Some(shared) = &self.shared {
            shared.state.lock().unwrap().target_fps = 1.0 / spf.as_secs_f64().max(f64::EPSILON);
        }
    }

    pub fn ack_required(&self, required: bool) {
        if let Some(shared) = &self.shared {
            shared.state.lock().unwrap().ack_required = required;
        }
    }

    pub fn encoded(&self, frame: &VideoFrame) {
        if let Some(shared) = &self.shared {
            let frames = match &frame.union {
                Some(
                    video_frame::Union::Vp8s(f)
                    | video_frame::Union::Vp9s(f)
                    | video_frame::Union::Av1s(f)
                    | video_frame::Union::H264s(f)
                    | video_frame::Union::H265s(f),
                ) => &f.frames,
                _ => return,
            };
            let mut state = shared.state.lock().unwrap();
            state.encoded_frames += frames.len();
            state.encoded_bytes += frames.iter().map(|f| f.data.len()).sum::<usize>();
        }
    }

    pub fn waiting(&self, sent: &HashSet<i32>, fetched: &HashSet<i32>) {
        if let Some(shared) = &self.shared {
            let mut state = shared.state.lock().unwrap();
            state.waiting_for.clear();
            state.waiting_for.extend(sent.difference(fetched).copied());
            state.waiting_for.sort_unstable();
        }
    }
}

impl Active {
    fn queue_age(&self, now: Instant) -> Duration {
        self.queued.map_or(Duration::ZERO, |queued| {
            now.saturating_duration_since(queued)
        })
    }
}

impl Shared {
    fn snapshot(&self, now: Instant) -> Snapshot {
        let mut state = self.state.lock().unwrap();
        Snapshot {
            active: state.active,
            stats: std::mem::take(&mut state.stats),
            timer_overdue: state
                .timers
                .iter()
                .map(|(&name, &due)| (name, now.saturating_duration_since(due)))
                .collect(),
            waiting_for: state.waiting_for.clone(),
            target_fps: state.target_fps,
            ack_required: state.ack_required,
            encoded_frames: std::mem::take(&mut state.encoded_frames),
            encoded_bytes: std::mem::take(&mut state.encoded_bytes),
        }
    }
}

impl Snapshot {
    fn log(scope: &str, snapshot: Self, window: Duration) {
        let now = Instant::now();
        let t = get_time();
        let a = snapshot.active;
        log::info!("qos_diag t={} scope={} event=heartbeat window_ms={} active={} kind={} display={} start_t={} active_ms={} frame_age_ms={} target_fps={:.1} encoded_frames={} encoded_fps={:.2} encoded_kbps={:.2} ack_required={} waiting_for={:?} timer_overdue_ms={:?}",
            t, scope, window.as_millis(), a.map_or("idle", |a| a.stage), a.map_or("none", |a| a.kind),
            a.map_or(-1, |a| a.display), a.map_or(0, |a| a.start_t),
            a.map_or(0, |a| now.saturating_duration_since(a.start).as_millis()),
            a.map_or(0, |a| a.queue_age(now).as_millis()), snapshot.target_fps,
            snapshot.encoded_frames, snapshot.encoded_frames as f64 / window.as_secs_f64(),
            snapshot.encoded_bytes as f64 * 0.008 / window.as_secs_f64(), snapshot.ack_required,
            snapshot.waiting_for, snapshot.timer_overdue.iter().map(|(name, age)| (*name, age.as_millis())).collect::<Vec<_>>());
        for ((stage, kind), s) in snapshot.stats {
            log::info!("qos_diag t={} scope={} event=summary window_ms={} stage={} kind={} count={} ok={} errors={} ok_per_s={:.2} avg_ms={:.2} max_ms={:.2} queue_max_ms={:.2}",
                t, scope, window.as_millis(), stage, kind, s.count, s.ok, s.errors,
                s.ok as f64 / window.as_secs_f64(), s.total.as_secs_f64() * 1000.0 / s.count as f64,
                s.max.as_secs_f64() * 1000.0, s.queue_max.as_secs_f64() * 1000.0);
        }
    }
}

impl Span {
    pub fn finish(&mut self, outcome: &'static str) {
        if let (Some(shared), Some(active)) = (&self.shared, self.active.take()) {
            let elapsed = active.start.elapsed();
            {
                let mut state = shared.state.lock().unwrap();
                state.active = self.previous;
                if active.stage == "frame_fetch_wait" {
                    state.waiting_for.clear();
                }
                let stats = state.stats.entry((active.stage, active.kind)).or_default();
                stats.count += 1;
                stats.ok += u64::from(outcome == "ok");
                stats.errors += u64::from(outcome == "error");
                stats.total += elapsed;
                stats.max = stats.max.max(elapsed);
                stats.queue_max = stats.queue_max.max(active.queue_age(active.start));
            }
            if elapsed >= SLOW || outcome == "error" || active.kind == "probe" {
                log::info!("qos_diag t={} scope={} event=end stage={} kind={} display={} start_t={} elapsed_ms={} queue_ms={} frame_age_ms={} outcome={}",
                    get_time(), shared.scope, active.stage, active.kind, active.display, active.start_t,
                    elapsed.as_millis(), active.queue_age(active.start).as_millis(), active.queue_age(Instant::now()).as_millis(), outcome);
            }
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        self.finish("returned");
    }
}

#[cfg(test)]
mod tests;
