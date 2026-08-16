use cocoa::{
    base::{id, nil, NO, YES},
    foundation::{NSPoint, NSRect, NSSize},
};
use dispatch::Queue;
use hbb_common::log;
use objc::{class, msg_send, sel, sel_impl};
use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Mutex,
    },
};

const DEFAULT_FRAME_COLOR: u32 = 0xFFA500;
const INACTIVE_FRAME_COLOR: u32 = 0x808080;
const DEFAULT_FRAME_WIDTH: u32 = 5;
const DEFAULT_FRAME_OPACITY: u32 = 50;
const CG_DISPLAY_BEGIN_CONFIGURATION_FLAG: u32 = 1 << 0;
const CG_SCREEN_SAVER_WINDOW_LEVEL_KEY: i32 = 13;
const NS_WINDOW_SHARING_READ_ONLY: usize = 1;

static REQUESTED_VISIBLE: AtomicBool = AtomicBool::new(false);
static FRAME_COLOR: AtomicU32 = AtomicU32::new(DEFAULT_FRAME_COLOR);
static FRAME_WIDTH: AtomicU32 = AtomicU32::new(DEFAULT_FRAME_WIDTH);
static FRAME_OPACITY: AtomicU32 = AtomicU32::new(DEFAULT_FRAME_OPACITY);
static DISPLAY_CALLBACK_REGISTERED: AtomicBool = AtomicBool::new(false);
static UPDATE_QUEUED: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    static ref WINDOWS: Mutex<Vec<ScreenFrameWindow>> = Mutex::new(Vec::new());
    static ref ACTIVE_DISPLAYS: Mutex<Vec<usize>> = Mutex::new(Vec::new());
}

type DisplayReconfigurationCallback = extern "C" fn(u32, u32, *mut c_void);

struct ScreenFrameWindow {
    panel: usize,
    window_id: u32,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGDisplayRegisterReconfigurationCallback(
        callback: DisplayReconfigurationCallback,
        user_info: *mut c_void,
    ) -> i32;
    fn CGWindowLevelForKey(key: i32) -> i32;
}

pub(crate) fn set_frame(
    visible: bool,
    color: u32,
    width: u32,
    opacity: u32,
    active_displays: &[usize],
) {
    if visible {
        register_display_callback();
    }
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
    if changed || color_changed || width_changed || opacity_changed || active_displays_changed {
        if unsafe { hbb_common::libc::pthread_main_np() } != 0 {
            apply_requested_state();
        } else {
            Queue::main().exec_sync(apply_requested_state);
        }
    }
}

pub(crate) fn window_ids() -> Vec<u32> {
    WINDOWS
        .lock()
        .unwrap()
        .iter()
        .map(|window| window.window_id)
        .collect()
}

fn register_display_callback() {
    if DISPLAY_CALLBACK_REGISTERED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    let result = unsafe {
        CGDisplayRegisterReconfigurationCallback(display_reconfigured, std::ptr::null_mut())
    };
    if result != 0 {
        DISPLAY_CALLBACK_REGISTERED.store(false, Ordering::Release);
        log::error!("Failed to monitor display changes for screen frame: {result}");
    }
}

extern "C" fn display_reconfigured(_display: u32, flags: u32, _user_info: *mut c_void) {
    if flags & CG_DISPLAY_BEGIN_CONFIGURATION_FLAG == 0 && REQUESTED_VISIBLE.load(Ordering::Acquire)
    {
        queue_update();
    }
}

fn queue_update() {
    if UPDATE_QUEUED.swap(true, Ordering::AcqRel) {
        return;
    }
    Queue::main().exec_async(apply_requested_state);
}

fn apply_requested_state() {
    UPDATE_QUEUED.store(false, Ordering::Release);
    objc::rc::autoreleasepool(|| unsafe {
        if !REQUESTED_VISIBLE.load(Ordering::Acquire) {
            destroy_windows();
        } else if !update_windows() {
            destroy_windows();
            create_windows();
        }
    });
    crate::ui_cm_interface::screen_frame_window_ids_changed(window_ids());
}

unsafe fn create_windows() {
    let Some(rects) = frame_rects() else {
        return;
    };
    let level = CGWindowLevelForKey(CG_SCREEN_SAVER_WINDOW_LEVEL_KEY) as i64;
    let active_displays = ACTIVE_DISPLAYS.lock().unwrap().clone();
    let mut windows = Vec::with_capacity(rects.len());
    for (index, rect) in rects.into_iter().enumerate() {
        let color = frame_color(active_displays.binary_search(&(index / 4)).is_ok());
        if let Some(window) = create_window(rect, level, color) {
            windows.push(window);
        }
    }
    WINDOWS.lock().unwrap().extend(windows);
}

unsafe fn frame_rects() -> Option<Vec<NSRect>> {
    let screens: id = msg_send![class!(NSScreen), screens];
    if screens == nil {
        log::error!("Failed to get displays for screen frame");
        return None;
    }
    let count: usize = msg_send![screens, count];
    if count == 0 {
        log::warn!("No display is available for screen frame");
        return None;
    }

    let frame_width = FRAME_WIDTH.load(Ordering::Acquire) as f64;
    let mut rects = Vec::with_capacity(count.saturating_mul(4));
    for index in 0..count {
        let screen: id = msg_send![screens, objectAtIndex: index];
        if screen == nil {
            continue;
        }
        let frame: NSRect = msg_send![screen, frame];
        let scale_factor: f64 = msg_send![screen, backingScaleFactor];
        if frame.size.width <= 0.0 || frame.size.height <= 0.0 {
            continue;
        }
        let scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        let thickness = (frame_width / scale_factor)
            .min(frame.size.width / 2.0)
            .min(frame.size.height / 2.0);
        let x = frame.origin.x;
        let y = frame.origin.y;
        let width = frame.size.width;
        let height = frame.size.height;
        rects.extend([
            NSRect::new(NSPoint::new(x, y), NSSize::new(width, thickness)),
            NSRect::new(
                NSPoint::new(x, y + height - thickness),
                NSSize::new(width, thickness),
            ),
            NSRect::new(NSPoint::new(x, y), NSSize::new(thickness, height)),
            NSRect::new(
                NSPoint::new(x + width - thickness, y),
                NSSize::new(thickness, height),
            ),
        ]);
    }
    Some(rects)
}

unsafe fn update_windows() -> bool {
    let Some(rects) = frame_rects() else {
        return false;
    };
    let windows = WINDOWS.lock().unwrap();
    if windows.len() != rects.len() {
        return false;
    }
    let active_displays = ACTIVE_DISPLAYS.lock().unwrap().clone();
    for (index, (window, rect)) in windows.iter().zip(rects).enumerate() {
        let panel = window.panel as id;
        let color = frame_color(active_displays.binary_search(&(index / 4)).is_ok());
        let _: () = msg_send![panel, setBackgroundColor: color];
        let _: () = msg_send![panel, setFrame: rect display: YES];
        let _: () = msg_send![panel, orderFrontRegardless];
    }
    true
}

unsafe fn frame_color(active: bool) -> id {
    let color = if active {
        FRAME_COLOR.load(Ordering::Acquire)
    } else {
        INACTIVE_FRAME_COLOR
    };
    let red = ((color >> 16) & 0xFF) as f64 / 255.0;
    let green = ((color >> 8) & 0xFF) as f64 / 255.0;
    let blue = (color & 0xFF) as f64 / 255.0;
    let alpha = FRAME_OPACITY.load(Ordering::Acquire) as f64 / 100.0;
    msg_send![class!(NSColor),
        colorWithSRGBRed: red
        green: green
        blue: blue
        alpha: alpha
    ]
}

unsafe fn create_window(rect: NSRect, level: i64, color: id) -> Option<ScreenFrameWindow> {
    let panel: id = msg_send![class!(NSPanel), alloc];
    if panel == nil {
        log::error!("Failed to allocate screen frame window");
        return None;
    }
    let panel: id = msg_send![panel,
        initWithContentRect: rect
        styleMask: 0usize
        backing: 2usize
        defer: NO
    ];
    if panel == nil {
        log::error!("Failed to initialize screen frame window");
        return None;
    }

    let collection_behavior = (1usize << 0) | (1usize << 4) | (1usize << 6) | (1usize << 8);
    let _: () = msg_send![panel, setBackgroundColor: color];
    let _: () = msg_send![panel, setOpaque: NO];
    let _: () = msg_send![panel, setHasShadow: NO];
    let _: () = msg_send![panel, setIgnoresMouseEvents: YES];
    let _: () = msg_send![panel, setHidesOnDeactivate: NO];
    let _: () = msg_send![panel, setCanHide: NO];
    let _: () = msg_send![panel, setSharingType: NS_WINDOW_SHARING_READ_ONLY];
    let _: () = msg_send![panel, setReleasedWhenClosed: NO];
    let _: () = msg_send![panel, setFloatingPanel: YES];
    let _: () = msg_send![panel, setBecomesKeyOnlyIfNeeded: YES];
    let _: () = msg_send![panel, setCollectionBehavior: collection_behavior];
    let _: () = msg_send![panel, setLevel: level];
    let _: () = msg_send![panel, orderFrontRegardless];
    let window_id: isize = msg_send![panel, windowNumber];
    if window_id <= 0 || window_id > u32::MAX as isize {
        log::error!("Failed to get a valid screen frame window number");
        let _: () = msg_send![panel, orderOut: nil];
        let _: () = msg_send![panel, release];
        return None;
    }
    Some(ScreenFrameWindow {
        panel: panel as usize,
        window_id: window_id as u32,
    })
}

unsafe fn destroy_windows() {
    let windows = std::mem::take(&mut *WINDOWS.lock().unwrap());
    for window in windows {
        let window = window.panel as id;
        let _: () = msg_send![window, orderOut: nil];
        let _: () = msg_send![window, release];
    }
}
