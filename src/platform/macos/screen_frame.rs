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
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

const FRAME_THICKNESS_PIXELS: f64 = 6.0;
const CG_DISPLAY_BEGIN_CONFIGURATION_FLAG: u32 = 1 << 0;
const CG_SCREEN_SAVER_WINDOW_LEVEL_KEY: i32 = 13;
const NS_WINDOW_SHARING_NONE: usize = 0;

static REQUESTED_VISIBLE: AtomicBool = AtomicBool::new(false);
static DISPLAY_CALLBACK_REGISTERED: AtomicBool = AtomicBool::new(false);
static UPDATE_QUEUED: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    static ref WINDOWS: Mutex<Vec<usize>> = Mutex::new(Vec::new());
}

type DisplayReconfigurationCallback = extern "C" fn(u32, u32, *mut c_void);

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGDisplayRegisterReconfigurationCallback(
        callback: DisplayReconfigurationCallback,
        user_info: *mut c_void,
    ) -> i32;
    fn CGWindowLevelForKey(key: i32) -> i32;
}

pub(crate) fn set_visible(visible: bool) {
    if visible {
        register_display_callback();
    }
    if REQUESTED_VISIBLE.swap(visible, Ordering::AcqRel) != visible {
        queue_update();
    }
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
        destroy_windows();
        if REQUESTED_VISIBLE.load(Ordering::Acquire) {
            create_windows();
        }
    });
}

unsafe fn create_windows() {
    let screens: id = msg_send![class!(NSScreen), screens];
    if screens == nil {
        log::error!("Failed to get displays for screen frame");
        return;
    }
    let count: usize = msg_send![screens, count];
    if count == 0 {
        log::warn!("No display is available for screen frame");
        return;
    }

    let level = CGWindowLevelForKey(CG_SCREEN_SAVER_WINDOW_LEVEL_KEY) as i64;
    let mut windows = Vec::with_capacity(count.saturating_mul(4));
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
        let thickness = (FRAME_THICKNESS_PIXELS / scale_factor)
            .min(frame.size.width / 2.0)
            .min(frame.size.height / 2.0);
        let x = frame.origin.x;
        let y = frame.origin.y;
        let width = frame.size.width;
        let height = frame.size.height;
        let rects = [
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
        ];
        for rect in rects {
            if let Some(window) = create_window(rect, level) {
                windows.push(window as usize);
            }
        }
    }
    WINDOWS.lock().unwrap().extend(windows);
}

unsafe fn create_window(rect: NSRect, level: i64) -> Option<id> {
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

    let color: id = msg_send![class!(NSColor),
        colorWithSRGBRed: 239.0f64 / 255.0
        green: 68.0f64 / 255.0
        blue: 56.0f64 / 255.0
        alpha: 0.75f64
    ];
    let collection_behavior = (1usize << 0) | (1usize << 4) | (1usize << 6) | (1usize << 8);
    let _: () = msg_send![panel, setBackgroundColor: color];
    let _: () = msg_send![panel, setOpaque: NO];
    let _: () = msg_send![panel, setHasShadow: NO];
    let _: () = msg_send![panel, setIgnoresMouseEvents: YES];
    let _: () = msg_send![panel, setHidesOnDeactivate: NO];
    let _: () = msg_send![panel, setCanHide: NO];
    let _: () = msg_send![panel, setSharingType: NS_WINDOW_SHARING_NONE];
    let _: () = msg_send![panel, setReleasedWhenClosed: NO];
    let _: () = msg_send![panel, setFloatingPanel: YES];
    let _: () = msg_send![panel, setBecomesKeyOnlyIfNeeded: YES];
    let _: () = msg_send![panel, setCollectionBehavior: collection_behavior];
    let _: () = msg_send![panel, setLevel: level];
    let _: () = msg_send![panel, orderFrontRegardless];
    Some(panel)
}

unsafe fn destroy_windows() {
    let windows = std::mem::take(&mut *WINDOWS.lock().unwrap());
    for window in windows {
        let window = window as id;
        let _: () = msg_send![window, orderOut: nil];
        let _: () = msg_send![window, release];
    }
}
