use base::{config::keys, message_proto::LoginMethods};
use hbb_common::{
    config::Config,
    password_security::{self as password, ApproveMode},
};

pub(super) fn available_login_methods() -> LoginMethods {
    let at_login_screen = crate::platform::is_prelogin();
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    let at_login_screen = at_login_screen || crate::platform::is_locked();
    #[cfg(target_os = "windows")]
    let at_login_screen = at_login_screen
        || crate::platform::is_logon_ui().unwrap_or_else(|err| {
            hbb_common::throttled_log!(
                std::time::Duration::from_secs(30),
                warn,
                "Failed to detect logon UI for login methods: {err}"
            );
            false
        });
    let allow_logon_password = at_login_screen
        && crate::get_builtin_option(keys::OPTION_ALLOW_LOGON_SCREEN_PASSWORD) == "Y";
    let mode = password::approve_mode();
    let has_password = password::has_valid_password();
    LoginMethods {
        click: mode != ApproveMode::Password && !at_login_screen,
        password: (mode != ApproveMode::Click || allow_logon_password)
            && (mode != ApproveMode::Both || has_password)
            && (has_password || allow_logon_password && Config::has_permanent_password()),
        ..Default::default()
    }
}
