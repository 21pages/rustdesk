use crate::{quartz, Frame, Pixfmt};
use hbb_common::{
    config::{keys::OPTION_ALLOW_SCREEN_FRAME, Config},
    log,
};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, TryLockError};
use std::time::{Duration, Instant};
use std::{io, mem};

const EXCLUDED_WINDOW_IDS_RETRY_INTERVAL: Duration = Duration::from_millis(100);

pub fn uses_screen_capture_kit() -> bool {
    screen_frame_enabled() && quartz::SckCapturer::is_available()
}

fn screen_frame_enabled() -> bool {
    Config::get_bool_option(OPTION_ALLOW_SCREEN_FRAME)
}

enum Backend {
    ScreenCaptureKit(quartz::SckCapturer),
    DisplayStream(quartz::Capturer),
}

impl Backend {
    fn width(&self) -> usize {
        match self {
            Self::ScreenCaptureKit(capturer) => capturer.width(),
            Self::DisplayStream(capturer) => capturer.width(),
        }
    }

    fn height(&self) -> usize {
        match self {
            Self::ScreenCaptureKit(capturer) => capturer.height(),
            Self::DisplayStream(capturer) => capturer.height(),
        }
    }
}

pub struct Capturer {
    inner: Backend,
    frame: Arc<Mutex<Option<quartz::Frame>>>,
    saved_raw_data: Vec<u8>, // for faster compare and copy
    excluded_window_ids_generation: Option<u64>,
    excluded_window_ids_pending_generation: Option<u64>,
    excluded_window_ids_retry: Option<(u64, Instant)>,
    screen_frame_enabled: bool,
}

impl Capturer {
    pub fn new(display: Display) -> io::Result<Capturer> {
        let frame = Arc::new(Mutex::new(None));
        let screen_frame_enabled = screen_frame_enabled();

        let (excluded_window_ids, excluded_window_ids_generation) = quartz::excluded_window_ids();
        let mut applied_excluded_window_ids_generation = Some(excluded_window_ids_generation);
        let inner = if screen_frame_enabled && quartz::SckCapturer::is_available() {
            let f = frame.clone();
            match quartz::SckCapturer::new(
                display.0,
                display.width(),
                display.height(),
                quartz::PixelFormat::Argb8888,
                false,
                &excluded_window_ids,
                move |inner| {
                    *f.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(inner);
                },
            ) {
                Ok(capturer) => {
                    if !capturer.initial_filter_ready() {
                        applied_excluded_window_ids_generation = None;
                    }
                    Backend::ScreenCaptureKit(capturer)
                }
                Err(err) => {
                    log::warn!(
                        "Failed to start ScreenCaptureKit, falling back to CGDisplayStream: {err}"
                    );
                    let f = frame.clone();
                    Backend::DisplayStream(
                        quartz::Capturer::new(
                            display.0,
                            display.width(),
                            display.height(),
                            quartz::PixelFormat::Argb8888,
                            Default::default(),
                            move |inner| {
                                *f.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) =
                                    Some(inner);
                            },
                        )
                        .map_err(|_| io::Error::from(io::ErrorKind::Other))?,
                    )
                }
            }
        } else {
            let f = frame.clone();
            Backend::DisplayStream(
                quartz::Capturer::new(
                    display.0,
                    display.width(),
                    display.height(),
                    quartz::PixelFormat::Argb8888,
                    Default::default(),
                    move |inner| {
                        *f.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(inner);
                    },
                )
                .map_err(|_| io::Error::from(io::ErrorKind::Other))?,
            )
        };

        Ok(Capturer {
            inner,
            frame,
            saved_raw_data: Vec::new(),
            excluded_window_ids_generation: applied_excluded_window_ids_generation,
            excluded_window_ids_pending_generation: None,
            excluded_window_ids_retry: None,
            screen_frame_enabled,
        })
    }

    pub fn width(&self) -> usize {
        self.inner.width()
    }

    pub fn height(&self) -> usize {
        self.inner.height()
    }
}

impl crate::TraitCapturer for Capturer {
    fn frame<'a>(&'a mut self, _timeout_ms: std::time::Duration) -> io::Result<Frame<'a>> {
        if self.screen_frame_enabled != screen_frame_enabled() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Screen frame capture mode changed",
            ));
        }
        if let Backend::ScreenCaptureKit(capturer) = &self.inner {
            if let Some(error) = capturer.stream_error() {
                return Err(io::Error::new(io::ErrorKind::Other, error));
            }
            let (window_ids, generation) = quartz::excluded_window_ids();
            if Some(generation) != self.excluded_window_ids_generation {
                let now = Instant::now();
                if self.excluded_window_ids_pending_generation.is_none()
                    && self
                        .excluded_window_ids_retry
                        .is_some_and(|(retry_generation, retry_at)| {
                            retry_generation == generation && now < retry_at
                        })
                {
                    return Err(io::ErrorKind::WouldBlock.into());
                }
                match capturer.update_excluded_window_ids(&window_ids)? {
                    quartz::ExcludedWindowUpdate::Pending => {
                        if self.excluded_window_ids_pending_generation.is_none() {
                            self.excluded_window_ids_pending_generation = Some(generation);
                        }
                    }
                    quartz::ExcludedWindowUpdate::Applied => {
                        let applied_generation = self
                            .excluded_window_ids_pending_generation
                            .take()
                            .unwrap_or(generation);
                        self.excluded_window_ids_generation = Some(applied_generation);
                        self.excluded_window_ids_retry = None;
                        *self
                            .frame
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
                    }
                    quartz::ExcludedWindowUpdate::NotReady => {
                        let retry_generation = self
                            .excluded_window_ids_pending_generation
                            .take()
                            .unwrap_or(generation);
                        self.excluded_window_ids_retry =
                            Some((retry_generation, now + EXCLUDED_WINDOW_IDS_RETRY_INTERVAL));
                    }
                }
                return Err(io::ErrorKind::WouldBlock.into());
            }
        }
        match self.frame.try_lock() {
            Ok(mut handle) => {
                let mut frame = None;
                mem::swap(&mut frame, &mut handle);

                match frame {
                    Some(mut frame) => {
                        crate::would_block_if_equal(&mut self.saved_raw_data, frame.inner())?;
                        frame.surface_to_bgra(self.height());
                        Ok(Frame::PixelBuffer(PixelBuffer {
                            frame,
                            data: PhantomData,
                            width: self.width(),
                            height: self.height(),
                        }))
                    }

                    None => Err(io::ErrorKind::WouldBlock.into()),
                }
            }

            Err(TryLockError::WouldBlock) => Err(io::ErrorKind::WouldBlock.into()),

            Err(TryLockError::Poisoned(..)) => Err(io::ErrorKind::Other.into()),
        }
    }
}

pub fn set_excluded_window_ids(window_ids: Vec<u32>) {
    quartz::set_excluded_window_ids(window_ids);
}

pub struct PixelBuffer<'a> {
    frame: quartz::Frame,
    data: PhantomData<&'a [u8]>,
    width: usize,
    height: usize,
}

impl<'a> crate::TraitPixelBuffer for PixelBuffer<'a> {
    fn data(&self) -> &[u8] {
        &*self.frame
    }

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn stride(&self) -> Vec<usize> {
        let mut v = Vec::new();
        v.push(self.frame.stride());
        v
    }

    fn pixfmt(&self) -> Pixfmt {
        Pixfmt::BGRA
    }
}

pub struct Display(quartz::Display);

impl Display {
    pub fn primary() -> io::Result<Display> {
        Ok(Display(quartz::Display::primary()))
    }

    pub fn all() -> io::Result<Vec<Display>> {
        Ok(quartz::Display::online()
            .map_err(|_| io::Error::from(io::ErrorKind::Other))?
            .into_iter()
            .map(Display)
            .collect())
    }

    pub fn width(&self) -> usize {
        self.0.width()
    }

    pub fn height(&self) -> usize {
        self.0.height()
    }

    pub fn scale(&self) -> f64 {
        self.0.scale()
    }

    pub fn name(&self) -> String {
        self.0.id().to_string()
    }

    pub fn is_online(&self) -> bool {
        self.0.is_online()
    }

    pub fn origin(&self) -> (i32, i32) {
        let o = self.0.bounds().origin;
        (o.x as _, o.y as _)
    }

    pub fn is_primary(&self) -> bool {
        self.0.is_primary()
    }
}
