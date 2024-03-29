#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

use crate::{config::Config, log};
use std::process::exit;

static mut GLOBAL_CALLBACK: Option<Box<dyn Fn()>> = None;

extern "C" fn breakdown_signal_handler(sig: i32) {
    log::error!("breakdown_signal_handler: {sig}");
    let mut stack = vec![];
    backtrace::trace(|frame| {
        backtrace::resolve_frame(frame, |symbol| {
            if let Some(name) = symbol.name() {
                stack.push(name.to_string());
            }
        });
        true // keep going to the next frame
    });
    log::error!("===================== begin print stack =====================");
    for s in &stack {
        log::error!("{}", s);
    }
    log::error!("===================== end print stack =====================");
    let mut info = String::default();
    if stack.iter().any(|s| {
        s.contains(&"nouveau_pushbuf_kick")
            || s.to_lowercase().contains("nvidia")
            || s.contains("gdk_window_end_draw_frame")
            || s.contains("glGetString")
    }) {
        Config::set_option("allow-always-software-render".to_string(), "Y".to_string());
        info = "Always use software rendering will be set.".to_string();
        log::info!("{}", info);
    }
    if stack.iter().any(|s| {
        s.to_lowercase().contains("nvidia")
            || s.to_lowercase().contains("amf")
            || s.to_lowercase().contains("mfx")
            || s.contains("cuProfilerStop")
    }) {
        Config::set_option("enable-hwcodec".to_string(), "N".to_string());
        info = "Perhaps hwcodec causing the crash, disable it first".to_string();
        log::info!("{}", info);
    }
    log::error!(
        "Got signal {} and exit. stack:\n{}",
        sig,
        stack.join("\n").to_string()
    );
    if !info.is_empty() {
        #[cfg(target_os = "linux")]
        linux::system_message(
            "RustDesk",
            &format!("Got signal {} and exit.{}", sig, info),
            true,
        )
        .ok();
    }
    unsafe {
        if let Some(callback) = &GLOBAL_CALLBACK {
            callback()
        }
    }
    exit(0);
}

pub fn register_breakdown_handler<T>(callback: T)
where
    T: Fn() + 'static,
{
    unsafe {
        GLOBAL_CALLBACK = Some(Box::new(callback));
        libc::signal(libc::SIGSEGV, breakdown_signal_handler as _);
    }
}
