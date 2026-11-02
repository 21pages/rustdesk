use crate::platform::screen_frame::Style;
use std::{
    mem,
    ptr::{null, null_mut},
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
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
            EnumDisplayMonitors, FillRect, GetClientRect, GetMessageW, GetMonitorInfoW,
            MonitorFromRect, MonitorFromWindow, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST, PeekMessageW, InvalidateRect,
            PostThreadMessageW, RegisterClassW, SetLayeredWindowAttributes,
            SetWindowDisplayAffinity, SetWindowPos, TranslateMessage, HTTRANSPARENT, HWND_TOPMOST,
            LWA_ALPHA, MA_NOACTIVATE, MSG, PAINTSTRUCT, PM_NOREMOVE, SWP_NOACTIVATE,
            SWP_SHOWWINDOW, WM_APP, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ERASEBKGND,
            WM_MOUSEACTIVATE, WM_NCHITTEST, WM_PAINT, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
};

const WM_SCREEN_FRAME_UPDATE: UINT = WM_APP + 1;
const WM_SCREEN_FRAME_REBUILD: UINT = WM_APP + 2;
const WM_SCREEN_FRAME_COLORS: UINT = WM_APP + 3;
const WDA_EXCLUDEFROMCAPTURE: DWORD = 0x0000_0011;
const CLASS_NAME: &[u16] = &[
    82, 117, 115, 116, 68, 101, 115, 107, 83, 99, 114, 101, 101, 110, 70, 114, 97, 109, 101, 0,
];

#[path = "screen_frame_controls.rs"]
mod controls;

static REQUESTED_VISIBLE: AtomicBool = AtomicBool::new(false);
static WORKER_RUNNING: AtomicBool = AtomicBool::new(false);
static REBUILD_QUEUED: AtomicBool = AtomicBool::new(false);
static WORKER_THREAD_ID: AtomicU32 = AtomicU32::new(0);

pub(crate) fn set_visible(visible: bool) {
    let changed = REQUESTED_VISIBLE.swap(visible, Ordering::AcqRel) != visible;
    if visible {
        ensure_worker();
    }
    if changed {
        notify_worker(WM_SCREEN_FRAME_UPDATE);
    } else if visible {
        // A remaining session can change the union of viewed displays without hiding the frame.
        refresh_colors();
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
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(5),
            error,
            "Failed to start screen frame thread: {err}"
        );
    }
}

fn notify_worker(message: UINT) -> bool {
    let thread_id = WORKER_THREAD_ID.load(Ordering::Acquire);
    if thread_id == 0 {
        return false;
    }
    if unsafe { PostThreadMessageW(thread_id, message, 0, 0) } == FALSE {
        let error = unsafe { GetLastError() };
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(5),
            debug,
            "Failed to notify screen frame thread: {error}"
        );
        return false;
    }
    true
}

fn request_rebuild() {
    if !REBUILD_QUEUED.swap(true, Ordering::AcqRel) && !notify_worker(WM_SCREEN_FRAME_REBUILD) {
        REBUILD_QUEUED.store(false, Ordering::Release);
    }
}

pub(crate) fn refresh() {
    if REQUESTED_VISIBLE.load(Ordering::Acquire) {
        request_rebuild();
    }
}

pub(crate) fn refresh_colors() {
    if REQUESTED_VISIBLE.load(Ordering::Acquire) {
        notify_worker(WM_SCREEN_FRAME_COLORS);
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
                    let error = GetLastError();
                    hbb_common::throttled_log!(
                        std::time::Duration::from_secs(5),
                        error,
                        "Screen frame message loop failed: {error}"
                    );
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
                    WM_SCREEN_FRAME_COLORS => {
                        // Repaint the same HWNDs so switching displays does not reset a dragged control.
                        for &window in &windows {
                            if InvalidateRect(window, null(), FALSE) == FALSE {
                                hbb_common::throttled_log!(std::time::Duration::from_secs(5), warn,
                                    "Failed to recolor screen frame: {}", GetLastError());
                            }
                        }
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
        let error = GetLastError();
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(5),
            error,
            "Failed to get the screen frame module: {error}"
        );
        return None;
    }
    let mut window_class: WNDCLASSW = mem::zeroed();
    window_class.lpfnWndProc = Some(window_proc);
    window_class.hInstance = instance;
    window_class.lpszClassName = CLASS_NAME.as_ptr();
    if RegisterClassW(&window_class) == 0 {
        let error = GetLastError();
        if error != ERROR_CLASS_ALREADY_EXISTS {
            hbb_common::throttled_log!(
                std::time::Duration::from_secs(5),
                error,
                "Failed to register the screen frame window: {error}"
            );
            return None;
        }
    }
    Some(instance)
}

unsafe fn apply_requested_state(instance: HINSTANCE, windows: &mut Vec<HWND>) {
    destroy_windows(windows);
    if REQUESTED_VISIBLE.load(Ordering::Acquire) {
        create_windows(instance, windows);
    }
}

unsafe fn create_windows(instance: HINSTANCE, windows: &mut Vec<HWND>) {
    let mut monitors: Vec<RECT> = Vec::new();
    if EnumDisplayMonitors(
        null_mut(),
        null(),
        Some(enum_monitor),
        &mut monitors as *mut Vec<RECT> as LPARAM,
    ) == FALSE
    {
        let error = GetLastError();
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(5),
            error,
            "Failed to enumerate displays for screen frame: {error}"
        );
        return;
    }

    windows.reserve(monitors.len().saturating_mul(5));
    for monitor in monitors {
        let style = style_for_monitor(MonitorFromRect(&monitor, MONITOR_DEFAULTTONEAREST));
        let width = monitor.right - monitor.left;
        let height = monitor.bottom - monitor.top;
        if width <= 0 || height <= 0 {
            continue;
        }
        let thickness = i32::from(style.width).min(width / 2).min(height / 2);
        if thickness <= 0 {
            continue;
        }
        let rects = [
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
        ];
        for rect in rects {
            if let Some(window) = create_window(instance, rect, &style) {
                windows.push(window);
            }
        }
        if let Some(window) = controls::create(instance, monitor) {
            windows.push(window);
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

unsafe fn create_window(instance: HINSTANCE, rect: RECT, style: &Style) -> Option<HWND> {
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
        let error = GetLastError();
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(5),
            error,
            "Failed to create a screen frame window: {error}"
        );
        return None;
    }
    let alpha = ((100 - u32::from(style.transparency)) * 255 / 100) as u8;
    if SetLayeredWindowAttributes(window, 0, alpha, LWA_ALPHA) == FALSE {
        let error = GetLastError();
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(5),
            error,
            "Failed to paint a screen frame window: {error}"
        );
        destroy_window(window);
        return None;
    }
    if base::platform::windows::is_windows_version_or_greater(10, 0, 19041, 0, 0)
        && SetWindowDisplayAffinity(window, WDA_EXCLUDEFROMCAPTURE) == FALSE
    {
        let error = GetLastError();
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(5),
            debug,
            "Failed to exclude a screen frame window from capture: {error}"
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
        let error = GetLastError();
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(5),
            error,
            "Failed to show a screen frame window: {error}"
        );
        destroy_window(window);
        return None;
    }
    Some(window)
}

unsafe fn destroy_windows(windows: &mut Vec<HWND>) {
    for window in windows.drain(..) {
        destroy_window(window);
    }
}

unsafe fn destroy_window(window: HWND) {
    if DestroyWindow(window) == FALSE {
        let error = GetLastError();
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(5),
            debug,
            "Failed to destroy a screen frame window: {error}"
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
                let color = style_for_monitor(MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST)).color;
                let brush = CreateSolidBrush(RGB(color[0], color[1], color[2]));
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

unsafe fn style_for_monitor(monitor: HMONITOR) -> Style {
    let mut info: MONITORINFOEXW = mem::zeroed();
    info.cbSize = mem::size_of::<MONITORINFOEXW>() as _;
    if GetMonitorInfoW(monitor, &mut info as *mut _ as _) == FALSE {
        hbb_common::throttled_log!(std::time::Duration::from_secs(5), warn,
            "Failed to identify screen frame monitor: {}", GetLastError());
        return Style::load();
    }
    let end = info.szDevice.iter().position(|&c| c == 0).unwrap_or(info.szDevice.len());
    Style::for_display(&String::from_utf16_lossy(&info.szDevice[..end]))
}
