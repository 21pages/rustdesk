// https://github.com/aarnt/qt-sudo

use crate::lang::translate;
use gtk::{glib, prelude::*};
use hbb_common::{
    anyhow::{bail, Error},
    log, ResultType,
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
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};

const ERR_PREFIX: &'static str = "sudo:";
const EXIT_CODE: i32 = -1;

pub fn run(cmds: Vec<&str>) -> ResultType<()> {
    // rustdesk service kill `rustdesk --` processes
    let mut args = vec!["gtk-sudo"];
    args.append(&mut cmds.clone());
    let mut child = crate::run_me(args)?;
    let exit_status = child.wait()?;
    if exit_status.success() {
        Ok(())
    } else {
        bail!("child exited with status: {:?}", exit_status);
    }
}

pub fn exec() {
    // https://docs.gtk.org/gtk4/ctor.Application.new.html
    // https://docs.gtk.org/gio/type_func.Application.id_is_valid.html
    let application = gtk::Application::new(None, Default::default());

    let (tx_to_ui, rx_to_ui) = channel::<Message>();
    let (tx_from_ui, rx_from_ui) = channel::<Message>();

    let rx_to_ui = Arc::new(Mutex::new(rx_to_ui));
    let tx_from_ui = Arc::new(Mutex::new(tx_from_ui));

    let rx_to_ui_clone = rx_to_ui.clone();
    let tx_from_ui_clone = tx_from_ui.clone();

    application.connect_activate(glib::clone!(@weak application =>move |_| {
        let rx_to_ui = rx_to_ui_clone.clone();
        let tx_from_ui = tx_from_ui_clone.clone();
        let last_password = Arc::new(Mutex::new(String::new()));

        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            if let Ok(msg) = rx_to_ui.lock().unwrap().try_recv() {
                match msg {
                    Message::PasswordPrompt(err_msg) => {
                        let last = last_password.lock().unwrap().clone();
                        if let Some(password) = password_prompt(&err_msg, &last) {
                            *last_password.lock().unwrap() = password.clone();
                            if let Err(e) = tx_from_ui
                                .lock()
                                .unwrap()
                                .send(Message::Password(password)) {
                                    error_dialog_and_exit(&format!("Channel error: {e:?}"), EXIT_CODE);
                                }
                        } else {
                        if let Err(e) = tx_from_ui.lock().unwrap().send(Message::Cancel) {
                                error_dialog_and_exit(&format!("Channel error: {e:?}"), EXIT_CODE);
                        }
                        }
                    }
                    Message::ErrorDialog(err_msg) => {
                        error_dialog_and_exit(&err_msg, EXIT_CODE);
                    }
                    Message::Exit(code) => {
                        log::info!("Exit code: {}", code);
                        std::process::exit(code);
                    }
                    _ => {}
                }
            }
            glib::ControlFlow::Continue
        });
    }));

    let tx_to_ui_clone = tx_to_ui.clone();
    let mut args = vec![];
    for arg in std::env::args().skip(2) {
        args.push(arg);
    }
    std::thread::spawn(move || match unsafe { forkpty(None, None) } {
        Ok(forkpty_result) => match forkpty_result {
            ForkptyResult::Parent { child, master } => {
                if let Err(e) = parent(child, master, tx_to_ui_clone, rx_from_ui) {
                    log::error!("Parent error: {:?}", e);
                    kill_child(child);
                    std::process::exit(EXIT_CODE);
                }
            }
            ForkptyResult::Child => {
                if let Err(e) = child(args) {
                    log::error!("Child error: {:?}", e);
                    std::process::exit(EXIT_CODE);
                }
            }
        },
        Err(err) => {
            log::error!("forkpty error: {:?}", err);
            if let Err(e) = tx_to_ui.send(Message::ErrorDialog(format!("Forkpty error: {:?}", err)))
            {
                log::error!("Channel error: {e:?}");
                std::process::exit(EXIT_CODE);
            }
        }
    });

    let _holder = application.hold();
    let args: Vec<&str> = vec![];
    application.run_with_args(&args);
    log::debug!("exit from gtk::Application::run_with_args");
    std::process::exit(EXIT_CODE);
}

enum Message {
    PasswordPrompt(String),
    Password(String),
    ErrorDialog(String),
    Cancel,
    Exit(i32),
}

fn parent(
    child: Pid,
    master: OwnedFd,
    tx_to_ui: Sender<Message>,
    rx_from_ui: Receiver<Message>,
) -> ResultType<()> {
    let raw_fd = master.as_raw_fd();
    if unsafe { fcntl(raw_fd, nix::libc::F_SETFL, nix::libc::O_NONBLOCK) } != 0 {
        let errno = std::io::Error::last_os_error();
        tx_to_ui.send(Message::ErrorDialog(format!("fcntl error: {errno:?}")))?;
        bail!("fcntl error: {errno:?}");
    }
    let mut file = unsafe { File::from_raw_fd(raw_fd) };

    let mut first = true;
    loop {
        let mut buf = [0; 1024];
        match file.read(&mut buf) {
            Ok(0) => {
                log::info!("read from child: EOF");
                break;
            }
            Ok(n) => {
                let buf = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                let last_line = buf.lines().last().unwrap_or(&buf).trim().to_string();
                log::info!("read from child: {}", buf);

                if last_line.starts_with(ERR_PREFIX) {
                    if let Err(e) = tx_to_ui.send(Message::ErrorDialog(last_line)) {
                        log::error!("Channel error: {e:?}");
                        kill_child(child);
                    }
                    break;
                } else if last_line.ends_with(":") {
                    match get_echo_turn_off(raw_fd) {
                        Ok(true) => {
                            log::debug!("get_echo_turn_off ok");
                            let err_msg = if first { "" } else { "Sorry, try again." };
                            first = false;
                            if let Err(e) =
                                tx_to_ui.send(Message::PasswordPrompt(err_msg.to_string()))
                            {
                                log::error!("Channel error: {e:?}");
                                kill_child(child);
                                break;
                            }
                            match rx_from_ui.recv() {
                                Ok(Message::Password(password)) => {
                                    let v = format!("{}\n", password);
                                    if let Err(e) = file.write_all(v.as_bytes()) {
                                        let e = format!("Failed to send password: {e:?}");
                                        if let Err(e) = tx_to_ui.send(Message::ErrorDialog(e)) {
                                            log::error!("Channel error: {e:?}");
                                        }
                                        kill_child(child);
                                        break;
                                    }
                                }
                                Ok(Message::Cancel) => {
                                    log::info!("User canceled");
                                    kill_child(child);
                                    break;
                                }
                                _ => {
                                    log::error!("Unexpected message");
                                    break;
                                }
                            }
                        }
                        Ok(false) => log::warn!("get_echo_turn_off timeout"),
                        Err(e) => log::error!("get_echo_turn_off error: {:?}", e),
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                // Child process is dead
                log::debug!("Read error: {:?}", e);
                break;
            }
        }
    }

    // Wait for child process
    let status = waitpid(child, None);
    log::info!("waitpid status: {:?}", status);
    let mut code = EXIT_CODE;
    match status {
        Ok(s) => match s {
            nix::sys::wait::WaitStatus::Exited(_pid, status) => {
                code = status;
            }
            _ => {}
        },
        Err(_) => {}
    }

    if let Err(e) = tx_to_ui.send(Message::Exit(code)) {
        log::error!("Channel error: {e:?}");
        std::process::exit(code);
    }
    Ok(())
}

fn child(args: Vec<String>) -> ResultType<()> {
    // https://doc.rust-lang.org/std/env/consts/constant.OS.html
    let os = std::env::consts::OS;
    let bsd = os == "freebsd" || os == "dragonfly" || os == "netbsd" || os == "openbad";
    let mut params = vec!["sudo".to_string()];
    params.push("/bin/sh".to_string());
    params.push("-c".to_string());
    let command = args
        .iter()
        .map(|arg| quote_shell_arg(arg))
        .collect::<Vec<String>>()
        .join(" ");

    let command = if bsd {
        let lc = match std::env::var("LC_ALL") {
            Ok(lc_all) => {
                if lc_all.contains('\'') {
                    eprintln!(
                        "{ERR_PREFIX} Detected attempt to inject privileged command via LC_ALL env({lc_all}). Exiting!\n",
                    );
                    std::process::exit(EXIT_CODE);
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
    let mut cparams = vec![];
    for param in &params {
        cparams.push(CString::new(param.as_str())?);
    }
    let res = execvp(CString::new("sudo")?.as_c_str(), &cparams);
    eprintln!("{ERR_PREFIX} execvp error: {:?}", res);
    std::process::exit(EXIT_CODE);
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
        log::info!("Force killing child process");
        unsafe { kill(child.as_raw(), Signal::SIGKILL as _) };
    }
}

fn password_prompt(err: &str, last_password: &str) -> Option<String> {
    let dialog = gtk::Dialog::builder()
        .title(crate::get_app_name())
        .modal(true)
        .build();
    // https://docs.gtk.org/gtk4/method.Dialog.set_default_response.html
    dialog.set_default_response(gtk::ResponseType::Ok);
    let content_area = dialog.content_area();

    let label = gtk::Label::builder()
        .label(translate("Authentication Required".to_string()))
        .margin_top(10)
        .build();
    content_area.add(&label);

    let image = gtk::Image::from_icon_name(Some("avatar-default-symbolic"), gtk::IconSize::Dialog);
    image.set_margin_top(10);
    content_area.add(&image);

    let user_label = gtk::Label::new(Some(&crate::platform::get_active_username()));
    content_area.add(&user_label);

    let password_input = gtk::Entry::builder()
        .visibility(false)
        .input_purpose(gtk::InputPurpose::Password)
        .placeholder_text(translate("Password".to_string()))
        .margin_top(20)
        .margin_start(30)
        .margin_end(30)
        .activates_default(true)
        .text(last_password)
        .build();
    // https://docs.gtk.org/gtk3/signal.Entry.activate.html
    password_input.connect_activate(glib::clone!(@weak dialog => move |_| {
        dialog.response(gtk::ResponseType::Ok);
    }));
    content_area.add(&password_input);

    if !err.is_empty() {
        let err_label = gtk::Label::new(None);
        err_label.set_markup(&format!(
            "<span font='10' foreground='orange'>{}</span>",
            err
        ));
        err_label.set_selectable(true);
        content_area.add(&err_label);
    }

    let cancel_button = gtk::Button::builder()
        .label(translate("Cancel".to_string()))
        .expand(true)
        .build();
    cancel_button.connect_clicked(glib::clone!(@weak dialog => move |_| {
        dialog.response(gtk::ResponseType::Cancel);
    }));
    let authenticate_button = gtk::Button::builder()
        .label(translate("Authenticate".to_string()))
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
        .margin_top(10)
        .build();
    button_box.add(&cancel_button);
    button_box.add(&authenticate_button);
    content_area.add(&button_box);

    content_area.set_spacing(10);
    content_area.set_border_width(10);

    dialog.set_width_request(400);
    dialog.show_all();
    dialog.set_position(gtk::WindowPosition::Center);
    dialog.set_keep_above(true);
    let response = dialog.run();
    dialog.hide();

    if response == gtk::ResponseType::Ok {
        Some(password_input.text().to_string())
    } else {
        None
    }
}

fn error_dialog_and_exit(err_msg: &str, exit_code: i32) {
    log::error!("Error dialog: {err_msg}, exit code: {exit_code}");
    let dialog = gtk::MessageDialog::builder()
        .message_type(gtk::MessageType::Error)
        .title(crate::get_app_name())
        .text("Error")
        .secondary_text(err_msg)
        .modal(true)
        .buttons(gtk::ButtonsType::Ok)
        .build();
    dialog.set_position(gtk::WindowPosition::Center);
    dialog.set_keep_above(true);
    dialog.run();
    dialog.close();
    std::process::exit(exit_code);
}

fn quote_shell_arg(arg: &str) -> String {
    let mut rv = arg.to_string();
    let re = hbb_common::regex::Regex::new("(\\s|[][!\"#$&'()*,;<=>?\\^`{}|~])");
    let Ok(re) = re else {
        return rv;
    };
    if re.is_match(arg) {
        rv = rv.replace("'", "'\\''");
        rv.insert(0, '\'');
        rv.push('\'');
    }
    rv
}
