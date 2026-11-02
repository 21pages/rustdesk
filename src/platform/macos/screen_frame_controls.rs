use super::{ScreenFrameWindow, Style, NS_WINDOW_SHARING_READ_ONLY};
use cocoa::{
    base::{id, nil, NO, YES},
    foundation::{NSPoint, NSRect, NSSize, NSString},
};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel, BOOL},
    sel, sel_impl,
};
use std::sync::OnceLock;

const WIDTH: f64 = 72.0;
const HEIGHT: f64 = 28.0;

pub(super) unsafe fn create(
    screen: NSRect,
    level: i64,
    style: &Style,
) -> Option<ScreenFrameWindow> {
    let view_class = control_class()?;
    if screen.size.width < WIDTH || screen.size.height < HEIGHT {
        return None;
    }
    let rect = NSRect::new(
        NSPoint::new(
            screen.origin.x + (screen.size.width - WIDTH) / 2.0,
            screen.origin.y + screen.size.height - HEIGHT,
        ),
        NSSize::new(WIDTH, HEIGHT),
    );
    // Set nonactivatingPanel during initialization so clicking never activates the CM app.
    let panel: id = msg_send![class!(NSPanel), alloc];
    let panel: id = msg_send![panel, initWithContentRect: rect
        styleMask: 1usize << 7 backing: 2usize defer: NO];
    if panel == nil {
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(5),
            error,
            "Failed to create screen frame control panel"
        );
        return None;
    }
    let view: id = msg_send![view_class, alloc];
    let view: id = msg_send![view, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), rect.size)];
    let button: id = msg_send![class!(NSButton), alloc];
    let button: id = msg_send![button, initWithFrame:
        NSRect::new(NSPoint::new(WIDTH - HEIGHT, 0.0), NSSize::new(HEIGHT, HEIGHT))];
    if view == nil || button == nil {
        let _: () = msg_send![view, release];
        let _: () = msg_send![button, release];
        let _: () = msg_send![panel, orderOut: nil];
        let _: () = msg_send![panel, release];
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(5),
            error,
            "Failed to create screen frame controls"
        );
        return None;
    }
    (*view).set_ivar("minX", screen.origin.x);
    (*view).set_ivar("maxX", screen.origin.x + screen.size.width - WIDTH);
    (*view).set_ivar("dragOffset", 0.0f64);
    // Controls must remain readable and clickable even with a thin, nearly transparent frame.
    let color: id = msg_send![class!(NSColor), colorWithSRGBRed: f64::from(style.color[0]) / 255.0
        green: f64::from(style.color[1]) / 255.0 blue: f64::from(style.color[2]) / 255.0 alpha: 1.0f64];
    let _: () = msg_send![panel, setBackgroundColor: color];
    let _: () = msg_send![panel, setHasShadow: NO];
    let _: () = msg_send![panel, setHidesOnDeactivate: NO];
    let _: () = msg_send![panel, setCanHide: NO];
    let _: () = msg_send![panel, setSharingType: NS_WINDOW_SHARING_READ_ONLY];
    let _: () = msg_send![panel, setReleasedWhenClosed: NO];
    let _: () = msg_send![panel, setFloatingPanel: YES];
    let _: () = msg_send![panel, setBecomesKeyOnlyIfNeeded: YES];
    let _: () = msg_send![panel, setCollectionBehavior:
        (1usize << 0) | (1usize << 4) | (1usize << 6) | (1usize << 8)];
    let _: () = msg_send![panel, setLevel: level];
    let _: () = msg_send![panel, setIgnoresMouseEvents: NO];
    let _: () = msg_send![panel, setContentView: view];
    let title = NSString::alloc(nil).init_str("×");
    let tooltip = NSString::alloc(nil).init_str(&crate::lang::translate("Disconnect".into()));
    let font: id = msg_send![class!(NSFont), systemFontOfSize: 20.0f64];
    let _: () = msg_send![button, setTitle: title];
    let _: () = msg_send![button, setToolTip: tooltip];
    let _: () = msg_send![button, setFont: font];
    let _: () = msg_send![button, setBordered: NO];
    let _: () = msg_send![button, setContentTintColor: foreground(style)];
    let _: () = msg_send![button, setTarget: view];
    let _: () = msg_send![button, setAction: sel!(disconnect:)];
    let _: () = msg_send![view, addSubview: button];
    let _: () = msg_send![title, release];
    let _: () = msg_send![tooltip, release];
    let _: () = msg_send![button, release];
    let _: () = msg_send![view, release];
    let _: () = msg_send![panel, orderFrontRegardless];
    let window_id: isize = msg_send![panel, windowNumber];
    if window_id <= 0 || window_id > u32::MAX as isize {
        let _: () = msg_send![panel, orderOut: nil];
        let _: () = msg_send![panel, release];
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(5),
            error,
            "Failed to get screen frame controls window ID"
        );
        return None;
    }
    Some(ScreenFrameWindow {
        panel: panel as usize,
        window_id: window_id as u32,
    })
}

fn control_class() -> Option<&'static Class> {
    static CLASS: OnceLock<Option<&'static Class>> = OnceLock::new();
    *CLASS.get_or_init(|| {
        let Some(mut decl) = ClassDecl::new("RustDeskScreenFrameControls", class!(NSView)) else {
            hbb_common::throttled_log!(
                std::time::Duration::from_secs(5),
                error,
                "Failed to register screen frame controls"
            );
            return None;
        };
        unsafe {
            decl.add_ivar::<f64>("minX");
            decl.add_ivar::<f64>("maxX");
            decl.add_ivar::<f64>("dragOffset");
            decl.add_method(
                sel!(acceptsFirstMouse:),
                accepts_first_mouse as extern "C" fn(&Object, Sel, id) -> BOOL,
            );
            decl.add_method(
                sel!(mouseDown:),
                mouse_down as extern "C" fn(&mut Object, Sel, id),
            );
            decl.add_method(
                sel!(mouseDragged:),
                mouse_dragged as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(sel!(drawRect:), draw as extern "C" fn(&Object, Sel, NSRect));
            decl.add_method(
                sel!(disconnect:),
                disconnect as extern "C" fn(&Object, Sel, id),
            );
        }
        Some(decl.register())
    })
}

pub(super) unsafe fn update_color(panel: id, style: &Style) {
    let view: id = msg_send![panel, contentView];
    let children: id = msg_send![view, subviews];
    let button: id = msg_send![children, firstObject];
    let _: () = msg_send![button, setContentTintColor: foreground(style)];
    let _: () = msg_send![view, setNeedsDisplay: YES];
}

extern "C" fn accepts_first_mouse(_: &Object, _: Sel, _: id) -> BOOL {
    YES
}

extern "C" fn mouse_down(view: &mut Object, _: Sel, event: id) {
    unsafe {
        let point: NSPoint = msg_send![event, locationInWindow];
        view.set_ivar("dragOffset", point.x);
    }
}

extern "C" fn mouse_dragged(view: &Object, _: Sel, _: id) {
    unsafe {
        let panel: id = msg_send![view, window];
        let frame: NSRect = msg_send![panel, frame];
        let mouse: NSPoint = msg_send![class!(NSEvent), mouseLocation];
        let x = (mouse.x - *view.get_ivar::<f64>("dragOffset"))
            .clamp(*view.get_ivar::<f64>("minX"), *view.get_ivar::<f64>("maxX"));
        // Move the same panel: its capture-exclusion ID stays valid throughout the drag.
        let _: () = msg_send![panel, setFrameOrigin: NSPoint::new(x, frame.origin.y)];
    }
}

unsafe fn foreground(style: &Style) -> id {
    let [r, g, b] = style.color.map(u32::from);
    if 299 * r + 587 * g + 114 * b < 128_000 {
        msg_send![class!(NSColor), whiteColor]
    } else {
        msg_send![class!(NSColor), blackColor]
    }
}

extern "C" fn draw(view: &Object, _: Sel, _: NSRect) {
    unsafe {
        let panel: id = msg_send![view, window];
        let screen: id = msg_send![panel, screen];
        let _: () = msg_send![foreground(&super::style_for_screen(screen)), setFill];
        for x in [18.0, 24.0] {
            for y in [7.0, 13.0, 19.0] {
                let _: () = msg_send![class!(NSBezierPath), fillRect:
                    NSRect::new(NSPoint::new(x, y), NSSize::new(2.0, 2.0))];
            }
        }
    }
}

extern "C" fn disconnect(_: &Object, _: Sel, _: id) {
    crate::ui_cm_interface::close_screen_sessions();
}
