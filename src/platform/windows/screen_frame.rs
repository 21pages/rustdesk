use hbb_common::log;
use std::{
    mem,
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Mutex,
    },
};
use winapi::{
    shared::{
        minwindef::{BOOL, DWORD, FALSE, HINSTANCE, LPARAM, LRESULT, TRUE, UINT, WPARAM},
        windef::{HDC, HMONITOR, HWND, LPRECT, RECT},
        winerror::ERROR_CLASS_ALREADY_EXISTS,
    },
    um::{
        errhandlingapi::GetLastError,
        libloaderapi::GetModuleHandleW,
        processthreadsapi::GetCurrentThreadId,
        wingdi::{CreateSolidBrush, DeleteObject, RGB},
        winuser::{
            BeginPaint, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, EndPaint,
            EnumDisplayMonitors, FillRect, GetClientRect, GetMessageW, GetWindowLongPtrW,
            InvalidateRect, PeekMessageW, PostThreadMessageW, RegisterClassW,
            SetLayeredWindowAttributes, SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos,
            TranslateMessage, GWLP_USERDATA, HTTRANSPARENT, HWND_TOPMOST, LWA_ALPHA, MA_NOACTIVATE,
            MSG, PAINTSTRUCT, PM_NOREMOVE, SWP_NOACTIVATE, SWP_SHOWWINDOW, WM_APP,
            WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ERASEBKGND, WM_MOUSEACTIVATE, WM_NCHITTEST,
            WM_PAINT, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
            WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
};

const DEFAULT_FRAME_COLOR: u32 = 0xFFA500;
const INACTIVE_FRAME_COLOR: u32 = 0x808080;
const DEFAULT_FRAME_WIDTH: u32 = 5;
const DEFAULT_FRAME_OPACITY: u32 = 50;
const WM_SCREEN_FRAME_UPDATE: UINT = WM_APP + 1;
const WM_SCREEN_FRAME_REBUILD: UINT = WM_APP + 2;
const WDA_EXCLUDEFROMCAPTURE: DWORD = 0x0000_0011;
const CLASS_NAME: &[u16] = &[
    82, 117, 115, 116, 68, 101, 115, 107, 83, 99, 114, 101, 101, 110, 70, 114, 97, 109, 101, 0,
];

static REQUESTED_VISIBLE: AtomicBool = AtomicBool::new(false);
static FRAME_COLOR: AtomicU32 = AtomicU32::new(DEFAULT_FRAME_COLOR);
static FRAME_WIDTH: AtomicU32 = AtomicU32::new(DEFAULT_FRAME_WIDTH);
static FRAME_OPACITY: AtomicU32 = AtomicU32::new(DEFAULT_FRAME_OPACITY);
static WORKER_RUNNING: AtomicBool = AtomicBool::new(false);
static REBUILD_QUEUED: AtomicBool = AtomicBool::new(false);
static WORKER_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static ACTIVE_DISPLAYS: Mutex<Vec<usize>> = Mutex::new(Vec::new());

pub(crate) fn set_frame(
    visible: bool,
    color: u32,
    width: u32,
    opacity: u32,
    active_displays: &[usize],
) {
    let changed = REQUESTED_VISIBLE.swap(visible, Ordering::AcqRel) != visible;
    let color = color & 0xFF_FFFF;
    let color_changed = FRAME_COLOR.swap(color, Ordering::AcqRel) != color;
    let width = width.clamp(5, 20);
    let width_changed = FRAME_WIDTH.swap(width, Ordering::AcqRel) != width;
    let opacity = opacity.clamp(20, 100);
    let opacity_changed = FRAME_OPACITY.swap(opacity, Ordering::AcqRel) != opacity;
    let active_displays_changed = {
        let mut displays = active_displays.to_vec();
        displays.sort_unstable();
        displays.dedup();
        let mut current = ACTIVE_DISPLAYS.lock().unwrap();
        if *current == displays {
            false
        } else {
            *current = displays;
            true
        }
    };
    if visible {
        ensure_worker();
    }
    if changed || color_changed || width_changed || opacity_changed || active_displays_changed {
        notify_worker(WM_SCREEN_FRAME_UPDATE);
    }
}

fn ensure_worker() {
    if WORKER_RUNNING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    if let Err(err) = std::thread::Builder::new()
        .name("screen-frame".to_owned())
        .spawn(worker_main)
    {
        WORKER_RUNNING.store(false, Ordering::Release);
        log::error!("Failed to start screen frame thread: {err}");
    }
}

fn notify_worker(message: UINT) -> bool {
    let thread_id = WORKER_THREAD_ID.load(Ordering::Acquire);
    if thread_id == 0 {
        return false;
    }
    if unsafe { PostThreadMessageW(thread_id, message, 0, 0) } == FALSE {
        log::debug!("Failed to notify screen frame thread: {}", unsafe {
            GetLastError()
        });
        return false;
    }
    true
}

fn request_rebuild() {
    if !REBUILD_QUEUED.swap(true, Ordering::AcqRel) && !notify_worker(WM_SCREEN_FRAME_REBUILD) {
        REBUILD_QUEUED.store(false, Ordering::Release);
    }
}

fn worker_main() {
    unsafe {
        let mut message: MSG = mem::zeroed();
        PeekMessageW(&mut message, null_mut(), 0, 0, PM_NOREMOVE);
        WORKER_THREAD_ID.store(GetCurrentThreadId(), Ordering::Release);

        let Some(instance) = register_window_class() else {
            WORKER_THREAD_ID.store(0, Ordering::Release);
            WORKER_RUNNING.store(false, Ordering::Release);
            return;
        };
        let mut windows = Vec::new();
        apply_requested_state(instance, &mut windows);

        loop {
            let result = GetMessageW(&mut message, null_mut(), 0, 0);
            if result <= 0 {
                if result < 0 {
                    log::error!("Screen frame message loop failed: {}", GetLastError());
                }
                break;
            }
            if message.hwnd.is_null() {
                match message.message {
                    WM_SCREEN_FRAME_UPDATE => apply_requested_state(instance, &mut windows),
                    WM_SCREEN_FRAME_REBUILD => {
                        REBUILD_QUEUED.store(false, Ordering::Release);
                        apply_requested_state(instance, &mut windows);
                    }
                    _ => {}
                }
            } else {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        destroy_windows(&mut windows);
    }
    WORKER_THREAD_ID.store(0, Ordering::Release);
    REBUILD_QUEUED.store(false, Ordering::Release);
    WORKER_RUNNING.store(false, Ordering::Release);
}

unsafe fn register_window_class() -> Option<HINSTANCE> {
    let instance = GetModuleHandleW(null());
    if instance.is_null() {
        log::error!("Failed to get the screen frame module: {}", GetLastError());
        return None;
    }
    let mut window_class: WNDCLASSW = mem::zeroed();
    window_class.lpfnWndProc = Some(window_proc);
    window_class.hInstance = instance;
    window_class.lpszClassName = CLASS_NAME.as_ptr();
    if RegisterClassW(&window_class) == 0 {
        let error = GetLastError();
        if error != ERROR_CLASS_ALREADY_EXISTS {
            log::error!("Failed to register the screen frame window: {error}");
            return None;
        }
    }
    Some(instance)
}

unsafe fn apply_requested_state(instance: HINSTANCE, windows: &mut Vec<HWND>) {
    if !REQUESTED_VISIBLE.load(Ordering::Acquire) {
        destroy_windows(windows);
        return;
    }
    let Some(rects) = frame_rects() else {
        return;
    };
    if windows.len() == rects.len() {
        update_windows(windows, &rects);
    } else {
        destroy_windows(windows);
        create_windows(instance, windows, rects);
    }
}

unsafe fn frame_rects() -> Option<Vec<RECT>> {
    let mut monitors: Vec<RECT> = Vec::new();
    if EnumDisplayMonitors(
        null_mut(),
        null(),
        Some(enum_monitor),
        &mut monitors as *mut Vec<RECT> as LPARAM,
    ) == FALSE
    {
        log::error!(
            "Failed to enumerate displays for screen frame: {}",
            GetLastError()
        );
        return None;
    }

    let frame_width = FRAME_WIDTH.load(Ordering::Acquire) as i32;
    let mut rects = Vec::with_capacity(monitors.len().saturating_mul(4));
    for monitor in monitors {
        let width = monitor.right - monitor.left;
        let height = monitor.bottom - monitor.top;
        if width <= 0 || height <= 0 {
            continue;
        }
        let thickness = frame_width.min(width / 2).min(height / 2);
        if thickness <= 0 {
            continue;
        }
        rects.extend([
            RECT {
                left: monitor.left,
                top: monitor.top,
                right: monitor.right,
                bottom: monitor.top + thickness,
            },
            RECT {
                left: monitor.left,
                top: monitor.bottom - thickness,
                right: monitor.right,
                bottom: monitor.bottom,
            },
            RECT {
                left: monitor.left,
                top: monitor.top,
                right: monitor.left + thickness,
                bottom: monitor.bottom,
            },
            RECT {
                left: monitor.right - thickness,
                top: monitor.top,
                right: monitor.right,
                bottom: monitor.bottom,
            },
        ]);
    }
    Some(rects)
}

unsafe fn create_windows(instance: HINSTANCE, windows: &mut Vec<HWND>, rects: Vec<RECT>) {
    windows.reserve(rects.len());
    for (index, rect) in rects.into_iter().enumerate() {
        if let Some(window) = create_window(instance, rect, index / 4) {
            windows.push(window);
        }
    }
}

unsafe fn update_windows(windows: &[HWND], rects: &[RECT]) {
    for (index, (window, rect)) in windows.iter().zip(rects).enumerate() {
        SetWindowLongPtrW(*window, GWLP_USERDATA, (index / 4 + 1) as _);
        if SetLayeredWindowAttributes(*window, 0, frame_alpha(), LWA_ALPHA) == FALSE {
            log::error!("Failed to update screen frame opacity: {}", GetLastError());
        }
        if SetWindowPos(
            *window,
            HWND_TOPMOST,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        ) == FALSE
        {
            log::error!("Failed to resize a screen frame window: {}", GetLastError());
        }
        if InvalidateRect(*window, null(), TRUE) == FALSE {
            log::debug!(
                "Failed to repaint a screen frame window: {}",
                GetLastError()
            );
        }
    }
}

unsafe extern "system" fn enum_monitor(
    _monitor: HMONITOR,
    _dc: HDC,
    rect: LPRECT,
    data: LPARAM,
) -> BOOL {
    if rect.is_null() || data == 0 {
        return TRUE;
    }
    let monitors = &mut *(data as *mut Vec<RECT>);
    monitors.push(*rect);
    TRUE
}

unsafe fn create_window(instance: HINSTANCE, rect: RECT, display_index: usize) -> Option<HWND> {
    let window = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
        CLASS_NAME.as_ptr(),
        null(),
        WS_POPUP,
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
        null_mut(),
        null_mut(),
        instance,
        null_mut(),
    );
    if window.is_null() {
        log::error!("Failed to create a screen frame window: {}", GetLastError());
        return None;
    }
    SetWindowLongPtrW(window, GWLP_USERDATA, (display_index + 1) as _);
    if SetLayeredWindowAttributes(window, 0, frame_alpha(), LWA_ALPHA) == FALSE {
        log::error!("Failed to paint a screen frame window: {}", GetLastError());
        destroy_window(window);
        return None;
    }
    if SetWindowDisplayAffinity(window, WDA_EXCLUDEFROMCAPTURE) == FALSE {
        log::debug!(
            "Failed to exclude a screen frame window from capture: {}",
            GetLastError()
        );
    }
    if SetWindowPos(
        window,
        HWND_TOPMOST,
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    ) == FALSE
    {
        log::error!("Failed to show a screen frame window: {}", GetLastError());
        destroy_window(window);
        return None;
    }
    Some(window)
}

fn frame_alpha() -> u8 {
    ((FRAME_OPACITY.load(Ordering::Acquire) * 255 + 50) / 100) as u8
}

fn frame_color(active: bool) -> u32 {
    if active {
        FRAME_COLOR.load(Ordering::Acquire)
    } else {
        INACTIVE_FRAME_COLOR
    }
}

unsafe fn destroy_windows(windows: &mut Vec<HWND>) {
    for window in windows.drain(..) {
        destroy_window(window);
    }
}

unsafe fn destroy_window(window: HWND) {
    if DestroyWindow(window) == FALSE {
        log::debug!(
            "Failed to destroy a screen frame window: {}",
            GetLastError()
        );
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match message {
        WM_NCHITTEST => HTTRANSPARENT,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_ERASEBKGND => 1,
        WM_DISPLAYCHANGE | WM_DPICHANGED => {
            request_rebuild();
            0
        }
        WM_PAINT => {
            let mut paint: PAINTSTRUCT = mem::zeroed();
            let dc = BeginPaint(window, &mut paint);
            let mut rect: RECT = mem::zeroed();
            if !dc.is_null() && GetClientRect(window, &mut rect) != FALSE {
                let display_index = GetWindowLongPtrW(window, GWLP_USERDATA);
                let active = display_index > 0
                    && ACTIVE_DISPLAYS
                        .lock()
                        .unwrap()
                        .binary_search(&((display_index - 1) as usize))
                        .is_ok();
                let color = frame_color(active);
                let brush = CreateSolidBrush(RGB(
                    ((color >> 16) & 0xFF) as u8,
                    ((color >> 8) & 0xFF) as u8,
                    (color & 0xFF) as u8,
                ));
                if !brush.is_null() {
                    FillRect(dc, &rect, brush);
                    DeleteObject(brush as _);
                }
            }
            EndPaint(window, &paint);
            0
        }
        _ => DefWindowProcW(window, message, w_param, l_param),
    }
}
