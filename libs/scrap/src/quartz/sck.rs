use std::{
    ffi::{c_char, c_void, CStr},
    io, ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use super::{Display, Frame, PixelFormat};

const ERROR_MESSAGE_CAPACITY: usize = 512;
const UPDATE_PENDING: i32 = 1;
const UPDATE_APPLIED: i32 = 2;
const UPDATE_NOT_READY: i32 = 3;

pub(crate) enum ExcludedWindowUpdate {
    Pending,
    Applied,
    NotReady,
}

#[repr(C)]
struct SckError {
    code: i32,
    message: [c_char; ERROR_MESSAGE_CAPACITY],
}

impl Default for SckError {
    fn default() -> Self {
        Self {
            code: 0,
            message: [0; ERROR_MESSAGE_CAPACITY],
        }
    }
}

type FrameCallback = unsafe extern "C" fn(*mut c_void, *mut c_void);
type ErrorCallback = unsafe extern "C" fn(*mut c_void, *const c_char);

extern "C" {
    fn scrap_sck_is_available() -> bool;
    fn scrap_sck_create(
        display_id: u32,
        width: usize,
        height: usize,
        cursor: bool,
        window_ids: *const u32,
        window_count: usize,
        filter_ready: *mut bool,
        frame_callback: FrameCallback,
        error_callback: ErrorCallback,
        callback_context: *mut c_void,
        error: *mut SckError,
    ) -> *mut c_void;
    fn scrap_sck_update_excluded_windows(
        handle: *mut c_void,
        window_ids: *const u32,
        window_count: usize,
        error: *mut SckError,
    ) -> i32;
    fn scrap_sck_destroy(handle: *mut c_void, error: *mut SckError) -> bool;
}

struct CallbackState {
    handler: Box<dyn Fn(Frame) + Send>,
    error: Arc<Mutex<Option<String>>>,
    filter_ready: Arc<AtomicBool>,
}

pub struct Capturer {
    handle: *mut c_void,
    callback_state: *mut CallbackState,
    error: Arc<Mutex<Option<String>>>,
    width: usize,
    height: usize,
    format: PixelFormat,
    display: Display,
    filter_ready: Arc<AtomicBool>,
}

impl Capturer {
    pub fn is_available() -> bool {
        unsafe { scrap_sck_is_available() }
    }

    pub fn new<F: Fn(Frame) + Send + 'static>(
        display: Display,
        width: usize,
        height: usize,
        format: PixelFormat,
        cursor: bool,
        excluded_window_ids: &[u32],
        handler: F,
    ) -> io::Result<Self> {
        let error = Arc::new(Mutex::new(None));
        let filter_ready = Arc::new(AtomicBool::new(false));
        let callback_state = Box::into_raw(Box::new(CallbackState {
            handler: Box::new(handler),
            error: error.clone(),
            filter_ready: filter_ready.clone(),
        }));
        let mut create_error = SckError::default();
        let mut initial_filter_ready = false;
        let (window_ids, window_count) = slice_parts(excluded_window_ids);
        let handle = unsafe {
            scrap_sck_create(
                display.id(),
                width,
                height,
                cursor,
                window_ids,
                window_count,
                &mut initial_filter_ready,
                frame_callback,
                error_callback,
                callback_state.cast(),
                &mut create_error,
            )
        };
        if handle.is_null() {
            unsafe {
                drop(Box::from_raw(callback_state));
            }
            return Err(error_from_ffi(&create_error));
        }
        filter_ready.store(initial_filter_ready, Ordering::Release);
        Ok(Self {
            handle,
            callback_state,
            error,
            width,
            height,
            format,
            display,
            filter_ready,
        })
    }

    pub(crate) fn update_excluded_window_ids(
        &self,
        excluded_window_ids: &[u32],
    ) -> io::Result<ExcludedWindowUpdate> {
        self.filter_ready.store(false, Ordering::Release);
        let mut update_error = SckError::default();
        let (window_ids, window_count) = slice_parts(excluded_window_ids);
        match unsafe {
            scrap_sck_update_excluded_windows(
                self.handle,
                window_ids,
                window_count,
                &mut update_error,
            )
        } {
            UPDATE_APPLIED => {
                self.filter_ready.store(true, Ordering::Release);
                Ok(ExcludedWindowUpdate::Applied)
            }
            UPDATE_PENDING => Ok(ExcludedWindowUpdate::Pending),
            UPDATE_NOT_READY => Ok(ExcludedWindowUpdate::NotReady),
            _ => Err(error_from_ffi(&update_error)),
        }
    }

    pub(crate) fn initial_filter_ready(&self) -> bool {
        self.filter_ready.load(Ordering::Acquire)
    }

    pub fn stream_error(&self) -> Option<String> {
        self.error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn format(&self) -> PixelFormat {
        self.format
    }

    pub fn display(&self) -> Display {
        self.display
    }
}

impl Drop for Capturer {
    fn drop(&mut self) {
        let mut destroy_error = SckError::default();
        let destroyed = unsafe { scrap_sck_destroy(self.handle, &mut destroy_error) };
        unsafe {
            drop(Box::from_raw(self.callback_state));
        }
        if !destroyed {
            hbb_common::log::error!(
                "ScreenCaptureKit shutdown did not complete safely: {}",
                error_from_ffi(&destroy_error)
            );
        }
    }
}

fn slice_parts(values: &[u32]) -> (*const u32, usize) {
    if values.is_empty() {
        (ptr::null(), 0)
    } else {
        (values.as_ptr(), values.len())
    }
}

fn error_from_ffi(error: &SckError) -> io::Error {
    let message = unsafe { CStr::from_ptr(error.message.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    let message = if message.is_empty() {
        format!("ScreenCaptureKit failed with error {}", error.code)
    } else {
        message
    };
    io::Error::new(io::ErrorKind::Other, message)
}

unsafe extern "C" fn frame_callback(context: *mut c_void, surface: *mut c_void) {
    if context.is_null() || surface.is_null() {
        return;
    }
    let state = &*(context as *const CallbackState);
    if !state.filter_ready.load(Ordering::Acquire) {
        return;
    }
    (state.handler)(Frame::new(surface));
}

unsafe extern "C" fn error_callback(context: *mut c_void, message: *const c_char) {
    if context.is_null() || message.is_null() {
        return;
    }
    let state = &*(context as *const CallbackState);
    *state
        .error
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) =
        Some(CStr::from_ptr(message).to_string_lossy().into_owned());
}

#[cfg(test)]
mod tests {
    use super::{scrap_sck_create, scrap_sck_destroy, scrap_sck_update_excluded_windows, Capturer};

    #[test]
    fn screen_capture_kit_availability_can_be_queried() {
        let _ = Capturer::is_available();
        let functions = [
            scrap_sck_create as usize,
            scrap_sck_update_excluded_windows as usize,
            scrap_sck_destroy as usize,
        ];
        assert!(functions.iter().all(|function| *function != 0));
    }
}
