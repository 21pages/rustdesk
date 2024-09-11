use gtk::{glib, prelude::*};
use hbb_common::{
    anyhow::{anyhow, bail, Error},
    log,
    regex::Regex,
    ResultType,
};
use nix::{
    libc::{fcntl, kill},
    pty::{forkpty, ForkptyResult},
    sys::{
        signal::Signal,
        wait::{waitpid, WaitPidFlag},
    },
    unistd::{execvp, setsid, Pid},
};
use std::{
    ffi::CString,
    fs::File,
    io::{Read, Write},
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
    sync::{Arc, Mutex},
};

struct PasswordDialog {
    dialog: gtk::Dialog,
    password_input: gtk::Entry,
}

impl PasswordDialog {
    fn new(user: &str, err: &str) -> Self {
        let dialog = gtk::Dialog::builder()
            .title("Authentication Required")
            .modal(true)
            .build();
        // https://docs.gtk.org/gtk4/method.Dialog.set_default_response.html
        dialog.set_default_response(gtk::ResponseType::Ok);
        let content_area = dialog.content_area();

        let label = gtk::Label::builder()
            .label("Authentication is required to change RustDesk options")
            .margin_top(10)
            .build();
        content_area.add(&label);

        let image =
            gtk::Image::from_icon_name(Some("avatar-default-symbolic"), gtk::IconSize::Dialog);
        image.set_margin_top(10);
        content_area.add(&image);

        let user_label = gtk::Label::new(Some(user));
        content_area.add(&user_label);

        let password_input = gtk::Entry::builder()
            .visibility(false)
            .input_purpose(gtk::InputPurpose::Password)
            .placeholder_text("Password")
            .margin_top(20)
            .margin_start(50)
            .margin_end(50)
            .activates_default(true)
            .build();
        // https://docs.gtk.org/gtk3/signal.Entry.activate.html
        password_input.connect_activate(glib::clone!(@weak dialog => move |_| {
            dialog.response(gtk::ResponseType::Ok);
        }));
        content_area.add(&password_input);

        if !err.is_empty() {
            let err_label = gtk::Label::new(None);
            err_label.set_markup(&format!("<span foreground='red'>{}</span>", err));
            content_area.add(&err_label);
        }

        let cancel_button = gtk::Button::builder().label("Cancel").expand(true).build();
        cancel_button.connect_clicked(glib::clone!(@weak dialog => move |_| {
            dialog.response(gtk::ResponseType::Cancel);
        }));
        let authenticate_button = gtk::Button::builder()
            .label("Authenticate")
            .expand(true)
            .build();
        authenticate_button.connect_clicked(glib::clone!(@weak dialog => move |_| {
            dialog.response(gtk::ResponseType::Ok);
        }));
        let button_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .expand(true)
            .homogeneous(true)
            .spacing(10)
            .build();
        button_box.add(&cancel_button);
        button_box.add(&authenticate_button);
        content_area.add(&button_box);

        content_area.set_spacing(10);
        content_area.set_border_width(10);

        PasswordDialog {
            dialog,
            password_input,
        }
    }

    fn exec(&self) -> Option<String> {
        self.dialog.show_all();
        self.dialog.set_position(gtk::WindowPosition::Center);
        self.dialog.set_keep_above(true);
        let response = self.dialog.run();
        self.dialog.hide();

        if response == gtk::ResponseType::Ok {
            Some(self.password_input.text().to_string())
        } else {
            None
        }
    }
}

pub fn run(command: &str) -> ResultType<()> {
    let mut child = crate::run_me(vec!["gtk-sudo", command])?;
    let exit_status = child.wait()?;
    log_to_file(&format!("gtk_sudo run exit_status: {exit_status:?}"));
    if exit_status.success() {
        Ok(())
    } else {
        bail!("child exited with status: {:?}", exit_status);
    }
}

pub fn exec(command: &str) {
    let mut ret = -1;
    match unsafe { forkpty(None, None) } {
        Ok(forkpty_result) => match forkpty_result {
            ForkptyResult::Parent { child, master } => match parent(child, master) {
                Ok(_) => {
                    ret = 0;
                }
                Err(e) => {
                    log::error!("parent error: {:?}", e);
                }
            },
            ForkptyResult::Child => {
                child(command).ok();
            }
        },
        Err(err) => {
            log::error!("forkpty error: {:?}", err);
        }
    }
    log_to_file(&format!("gtk_sudo exec ret: {ret}"));
    std::process::exit(ret);
}

fn parent(child: Pid, master: OwnedFd) -> ResultType<()> {
    let raw_fd = master.as_raw_fd();
    unsafe { fcntl(raw_fd, nix::libc::F_SETFL, nix::libc::O_NONBLOCK) };
    let mut file = unsafe { File::from_raw_fd(raw_fd) };

    let child_finished = Arc::new(Mutex::new(false));
    let child_finished_cloned = child_finished.clone();
    // read from child
    let read_err = Arc::new(Mutex::new(String::default()));
    let read_err_cloned = read_err.clone();
    let handle = std::thread::spawn(move || {
        let mut first = true;
        loop {
            let mut buf = [0; 1024];
            match file.read(&mut buf) {
                Ok(0) => {
                    break;
                }
                Ok(n) => {
                    let buf = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                    let mut last_line = buf.lines().last().unwrap_or(&buf).trim().to_string();
                    if last_line.is_empty() {
                        last_line = buf.clone();
                    }
                    log::info!("read from child: {}", last_line);
                    log_to_file(&format!("buf: {buf}"));
                    log_to_file(&format!("last_line: {last_line}"));
                    if last_line.starts_with("sudo:") {
                        *read_err_cloned.lock().unwrap() = last_line.clone();
                        error_dialog(last_line);
                        break;
                    } else if last_line.ends_with(":") {
                        match get_echo_turn_off(raw_fd) {
                            Ok(true) => {
                                log_to_file("get_echo_turn_off ok");
                                let err_msg = if first { "" } else { "Sorry, try again." };
                                first = false;
                                let Some(password) =
                                    block_get_password_from_dialog(err_msg.to_string())
                                else {
                                    kill_child(child);
                                    break;
                                };
                                let v = format!("{}\n", password);
                                if let Err(e) = file.write_all(v.as_bytes()) {
                                    log::error!("write password to child process error: {:?}", e);
                                    break;
                                }
                            }
                            Ok(false) => {
                                log::warn!("get_echo_turn_off timeout");
                            }
                            Err(e) => {
                                log::error!("get_echo_turn_off error: {:?}", e);
                            }
                        }
                    }
                }
                Err(_) => {
                    if child_finished_cloned.lock().unwrap().clone() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
    });

    // wait child
    let status = waitpid(child, None);
    log_to_file(&format!("waitpid status: {:?}", status));
    let mut wait_err = String::default();
    match status {
        Ok(s) => match s {
            nix::sys::wait::WaitStatus::Exited(pid, status) => {
                if child == pid && status == 0 {
                } else {
                    wait_err = format!("wait status: {s:?}");
                }
            }
            _ => {
                wait_err = format!("wait error: {s:?}");
            }
        },
        Err(e) => wait_err = format!("wait errno: {e:?}"),
    }
    *child_finished.lock().unwrap() = true;
    if wait_err.is_empty() {
        Ok(())
    } else {
        // wait to read the last line
        std::thread::sleep(std::time::Duration::from_millis(50));
        let read_err = read_err.lock().unwrap().clone();
        if !read_err.is_empty() {
            handle.join().ok();
        }
        bail!(wait_err);
    }
}

fn child(command: &str) -> ResultType<()> {
    // https://doc.rust-lang.org/std/env/consts/constant.OS.html
    let os = std::env::consts::OS;
    let bsd = os == "freebsd" || os == "dragonfly" || os == "netbsd" || os == "openbad";
    let mut params = vec!["sudo".to_string()];
    // params.push(format!("--preserve-env={}", env_workarounds()));
    params.push("/bin/sh".to_string());
    params.push("-c".to_string());

    let command = if bsd {
        let lc = match std::env::var("LC_ALL") {
            Ok(lc_all) => {
                if lc_all.contains('\'') {
                    eprintln!(
                        "sudo: Detected attempt to inject privileged command via LC_ALL env({}). Exiting!\n",
                        lc_all
                    );
                    std::process::exit(-4);
                }
                format!("LC_ALL='{lc_all}' ")
            }
            Err(_) => {
                format!("unset LC_ALL;")
            }
        };
        format!("{}exec {}", lc, command)
    } else {
        command.to_string()
    };
    params.push(command);
    std::env::set_var("LC_ALL", "C.UTF-8");

    // allow failure here
    let _ = setsid();
    log_to_file(&format!("params: {:?}", params));
    let mut cparams = vec![];
    for param in &params {
        cparams.push(CString::new(param.as_str())?);
    }
    let res = execvp(CString::new("sudo")?.as_c_str(), &cparams);
    log_to_file(&format!("execvp res: {res:?}"));
    eprintln!("sudo: execvp error: {:?}", std::io::Error::last_os_error());
    std::process::exit(2);
}

fn block_get_password_from_dialog(err: String) -> Option<String> {
    let password = Arc::new(Mutex::new(None));
    let password_clone = password.clone();
    let application = gtk::Application::new(Some("com.rustdesk.RustDesk"), Default::default());
    application.connect_activate(move |_| {
        let dialog = PasswordDialog::new(&crate::platform::get_active_username(), &err);
        if let Some(password) = dialog.exec() {
            *password_clone.lock().unwrap() = Some(password);
        } else {
            *password_clone.lock().unwrap() = None;
        }
    });
    // https://gtk-rs.org/gtk-rs-core/stable/0.14/docs/gio/prelude/trait.ApplicationExtManual.html#tymethod.run_with_args
    let args: Vec<&str> = vec![];
    application.run_with_args(&args);
    let password = password.lock().unwrap().clone();
    password
}

fn error_dialog(err: String) {
    let application = gtk::Application::new(Some("com.rustdesk.RustDesk"), Default::default());
    application.connect_activate(move |_| {
        let dialog = gtk::MessageDialog::builder()
            .message_type(gtk::MessageType::Error)
            .title("Error")
            .text(&err)
            .modal(true)
            .buttons(gtk::ButtonsType::Ok)
            .build();
        dialog.set_position(gtk::WindowPosition::Center);
        dialog.set_keep_above(true);
        dialog.run();
        dialog.close();
    });
    let args: Vec<&str> = vec![];
    application.run_with_args(&args);
}

fn get_echo_turn_off(fd: RawFd) -> Result<bool, Error> {
    let tios = termios::Termios::from_fd(fd)?;
    for _ in 0..10 {
        if tios.c_lflag & termios::ECHO == 0 {
            return Ok(true);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Ok(false)
}

fn kill_child(child: Pid) {
    unsafe { kill(child.as_raw(), Signal::SIGINT as _) };
    let mut res = 0;

    for _ in 0..10 {
        match waitpid(child, Some(WaitPidFlag::WNOHANG)) {
            Ok(_) => {
                res = 1;
                break;
            }
            Err(_) => (),
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    if res == 0 {
        unsafe { kill(child.as_raw(), Signal::SIGKILL as _) };
    }
}

// fn env_workarounds() -> String {
//     const ALLOWED_VARS: [&str; 28] = [
//         "DISPLAY",
//         "GDK_DPI_SCALE",
//         "GDK_SCALE",
//         "GTK_CSD",
//         "GTK_OVERLAY_SCROLLING",
//         "LANG",
//         "LANGUAGE",
//         "LC_ADDRESS",
//         "LC_ALL",
//         "LC_COLLATE",
//         "LC_CTYPE",
//         "LC_IDENTIFICATION",
//         "LC_MEASUREMENT",
//         "LC_MESSAGES",
//         "LC_MONETARY",
//         "LC_NAME",
//         "LC_NUMERIC",
//         "LC_PAPER",
//         "LC_TELEPHONE",
//         "LC_TIME",
//         "PATH",
//         "QT_PLATFORM_PLUGIN",
//         "QT_QPA_PLATFORMTHEME",
//         "QT_SCALE_FACTOR",
//         "TERM",
//         "WAYLAND_DISPLAY",
//         "XAUTHLOCALHOSTNAME",
//         "XAUTHORITY",
//     ];

//     for (key, _) in std::env::vars() {
//         if !ALLOWED_VARS.contains(&key.as_str()) {
//             std::env::remove_var(&key);
//         }
//     }

//     ALLOWED_VARS.join(", ")
// }

// fn quote_shell_arg(arg: &str, user_friendly: bool) -> String {
//     let mut rv = arg.to_string();
//     let re = Regex::new("(\\s|[][!\"#$&'()*,;<=>?\\^`{}|~])");
//     let Ok(re) = re else {
//         return rv;
//     };
//     if !user_friendly || re.is_match(arg) {
//         rv = rv.replace("'", "'\\''");
//         rv.insert(0, '\'');
//         rv.push('\'');
//     }
//     rv
// }

fn log_to_file(s: &str) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("/tmp/hello.txt")
        .unwrap();
    file.write_all(s.as_bytes()).unwrap();
    file.write(b"\n").unwrap();
}
