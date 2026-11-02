use super::{destroy_window, request_rebuild, WDA_EXCLUDEFROMCAPTURE};
use std::{mem, ptr::null_mut};
use winapi::{
    shared::{
        minwindef::{FALSE, HINSTANCE, LPARAM, LRESULT, TRUE, UINT, WPARAM},
        windef::{HWND, POINT, RECT},
        winerror::ERROR_CLASS_ALREADY_EXISTS,
    },
    um::{
        errhandlingapi::{GetLastError, SetLastError},
        libloaderapi::{GetModuleHandleA, GetProcAddress},
        wingdi::{CreateSolidBrush, DeleteObject, GetDeviceCaps, LOGPIXELSX, RGB},
        winuser::{
            BeginPaint, CreateWindowExW, DefWindowProcW, DrawFrameControl, EndPaint, FillRect,
            GetCapture, GetClientRect, GetCursorPos, GetDC, GetMonitorInfoW, GetWindowLongPtrW,
            GetWindowRect, MonitorFromWindow, RegisterClassW, ReleaseCapture, ReleaseDC,
            SetCapture, SetLayeredWindowAttributes, SetWindowDisplayAffinity, SetWindowLongPtrW,
            SetWindowPos, BN_CLICKED, BS_OWNERDRAW, DFCS_CAPTIONCLOSE, DFCS_PUSHED, DFC_CAPTION,
            DRAWITEMSTRUCT, GWLP_USERDATA, HTCLIENT, HWND_TOPMOST, LWA_ALPHA, MA_NOACTIVATE,
            MK_LBUTTON, MONITORINFO, MONITOR_DEFAULTTONEAREST, ODS_SELECTED, PAINTSTRUCT,
            SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, WM_COMMAND, WM_DISPLAYCHANGE,
            WM_DPICHANGED, WM_DRAWITEM, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP,
            WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCHITTEST, WM_PAINT, WNDCLASSW, WS_CHILD,
            WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
        },
    },
};

const CLOSE_BUTTON: usize = 1;

pub(super) unsafe fn create(instance: HINSTANCE, monitor: RECT) -> Option<HWND> {
    let class: Vec<u16> = "RustDeskScreenFrameControls\0".encode_utf16().collect();
    let mut definition: WNDCLASSW = mem::zeroed();
    definition.lpfnWndProc = Some(window_proc);
    definition.hInstance = instance;
    definition.lpszClassName = class.as_ptr();
    if RegisterClassW(&definition) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
        report_error("register controls");
        return None;
    }
    let title: Vec<u16> = crate::lang::translate("Disconnect".into())
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let window = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
        class.as_ptr(),
        title.as_ptr(),
        WS_POPUP,
        monitor.left + (monitor.right - monitor.left) / 2,
        monitor.top,
        72,
        28,
        null_mut(),
        null_mut(),
        instance,
        null_mut(),
    );
    if window.is_null() {
        report_error("create controls");
        return None;
    }
    let dpi = window_dpi(window);
    let width = (72 * dpi / 96).min(monitor.right - monitor.left);
    let height = (28 * dpi / 96).min(monitor.bottom - monitor.top);
    let button_class: Vec<u16> = "BUTTON\0".encode_utf16().collect();
    let button_title: Vec<u16> = "×\0".encode_utf16().collect();
    let button = CreateWindowExW(
        0,
        button_class.as_ptr(),
        button_title.as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_OWNERDRAW,
        width - height,
        0,
        height,
        height,
        window,
        CLOSE_BUTTON as _,
        instance,
        null_mut(),
    );
    // Keep the controls readable regardless of the frame's width and transparency settings.
    if button.is_null() || SetLayeredWindowAttributes(window, 0, 255, LWA_ALPHA) == FALSE {
        report_error("initialize controls");
        destroy_window(window);
        return None;
    }
    if base::platform::windows::is_windows_version_or_greater(10, 0, 19041, 0, 0)
        && SetWindowDisplayAffinity(window, WDA_EXCLUDEFROMCAPTURE) == FALSE
    {
        report_error("exclude controls from capture");
    }
    if SetWindowPos(
        window,
        HWND_TOPMOST,
        monitor.left + (monitor.right - monitor.left - width) / 2,
        monitor.top,
        width,
        height,
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    ) == FALSE
    {
        report_error("show controls");
        destroy_window(window);
        return None;
    }
    Some(window)
}

unsafe fn window_dpi(window: HWND) -> i32 {
    // GetDpiForWindow is absent on Windows 7; do not add a mandatory newer User32 import.
    let module = GetModuleHandleA(b"user32.dll\0".as_ptr() as _);
    if !module.is_null() {
        let address = GetProcAddress(module, b"GetDpiForWindow\0".as_ptr() as _);
        if !address.is_null() {
            let get_dpi: unsafe extern "system" fn(HWND) -> UINT = mem::transmute(address);
            let dpi = get_dpi(window);
            if dpi != 0 {
                return dpi as i32;
            }
        }
    }
    let dc = GetDC(window);
    if dc.is_null() {
        return 96;
    }
    let dpi = GetDeviceCaps(dc, LOGPIXELSX);
    ReleaseDC(window, dc);
    if dpi > 0 {
        dpi
    } else {
        96
    }
}

unsafe fn report_error(operation: &str) {
    let error = GetLastError();
    hbb_common::throttled_log!(
        std::time::Duration::from_secs(5),
        warn,
        "Failed to {operation} for screen frame: {error}"
    );
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match message {
        WM_NCHITTEST => HTCLIENT,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_ERASEBKGND => 1,
        WM_LBUTTONDOWN => {
            // Stay in the worker's message loop while dragging so hide/rebuild requests are handled.
            SetLastError(0);
            if SetWindowLongPtrW(window, GWLP_USERDATA, l_param as i16 as isize) == 0
                && GetLastError() != 0
            {
                report_error("start dragging controls");
                return 0;
            }
            SetCapture(window);
            if GetCapture() != window {
                hbb_common::throttled_log!(
                    std::time::Duration::from_secs(5),
                    warn,
                    "Failed to capture mouse for screen frame controls"
                );
            }
            0
        }
        WM_LBUTTONUP => {
            if GetCapture() == window && ReleaseCapture() == FALSE {
                report_error("release controls mouse capture");
            }
            0
        }
        WM_COMMAND if w_param & 0xffff == CLOSE_BUTTON && (w_param >> 16) as u16 == BN_CLICKED => {
            crate::ui_cm_interface::close_screen_sessions();
            0
        }
        WM_DRAWITEM if w_param == CLOSE_BUTTON && l_param != 0 => {
            // Use the native close glyph at the button's DPI-scaled size, without owning a font.
            let item = &*(l_param as *const DRAWITEMSTRUCT);
            let pressed = if item.itemState & ODS_SELECTED != 0 {
                DFCS_PUSHED
            } else {
                0
            };
            let mut rect = item.rcItem;
            if DrawFrameControl(
                item.hDC,
                &mut rect,
                DFC_CAPTION,
                DFCS_CAPTIONCLOSE | pressed,
            ) == FALSE
            {
                report_error("draw close button");
            }
            TRUE as LRESULT
        }
        WM_MOUSEMOVE if GetCapture() == window && w_param & MK_LBUTTON as usize != 0 => {
            let mut info: MONITORINFO = mem::zeroed();
            info.cbSize = mem::size_of::<MONITORINFO>() as _;
            let mut rect: RECT = mem::zeroed();
            let mut mouse: POINT = mem::zeroed();
            if GetMonitorInfoW(
                MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST),
                &mut info,
            ) != FALSE
                && GetWindowRect(window, &mut rect) != FALSE
                && GetCursorPos(&mut mouse) != FALSE
            {
                let width = rect.right - rect.left;
                let x = (mouse.x - GetWindowLongPtrW(window, GWLP_USERDATA) as i32).clamp(
                    info.rcMonitor.left,
                    (info.rcMonitor.right - width).max(info.rcMonitor.left),
                );
                if SetWindowPos(
                    window,
                    null_mut(),
                    x,
                    info.rcMonitor.top,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
                ) == FALSE
                {
                    report_error("move controls");
                }
            } else {
                report_error("locate controls monitor");
            }
            0
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED => {
            request_rebuild();
            0
        }
        WM_PAINT => {
            let mut paint: PAINTSTRUCT = mem::zeroed();
            let dc = BeginPaint(window, &mut paint);
            let mut rect: RECT = mem::zeroed();
            if !dc.is_null() && GetClientRect(window, &mut rect) != FALSE {
                let [r, g, b] = super::style_for_monitor(MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST)).color;
                let background = CreateSolidBrush(RGB(r, g, b));
                if !background.is_null() {
                    FillRect(dc, &rect, background);
                    DeleteObject(background as _);
                }
                let light = 299 * u32::from(r) + 587 * u32::from(g) + 114 * u32::from(b) < 128_000;
                let ink = if light { 255 } else { 0 };
                let brush = CreateSolidBrush(RGB(ink, ink, ink));
                if !brush.is_null() {
                    let scale = f64::from(rect.bottom) / 28.0;
                    for x in [18.0, 24.0] {
                        for y in [7.0, 13.0, 19.0] {
                            let dot = RECT {
                                left: (x * scale) as _,
                                top: (y * scale) as _,
                                right: ((x + 2.0) * scale) as _,
                                bottom: ((y + 2.0) * scale) as _,
                            };
                            FillRect(dc, &dot, brush);
                        }
                    }
                    DeleteObject(brush as _);
                }
            }
            EndPaint(window, &paint);
            0
        }
        _ => DefWindowProcW(window, message, w_param, l_param),
    }
}
