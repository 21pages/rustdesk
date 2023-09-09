use crate::{client::translate, ipc::Data};
use hbb_common::{allow_err, log, tokio, ResultType};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[cfg(all(windows, feature = "flutter"))]
pub fn check_and_start_tray_process() {
    std::thread::spawn(|| {
        if let Err(e) = check_and_start_tray_process_exec() {
            log::error!("start tray failed:{:?}", e);
        }
    });
}

#[cfg(all(windows, feature = "flutter"))]
#[tokio::main(flavor = "current_thread")]
async fn check_and_start_tray_process_exec() -> ResultType<()> {
    let mut server_exist = None;
    let mut tray_exist = None;
    let is_root = crate::platform::is_root();
    if is_root {
        server_exist = Some(crate::check_process("--server", false, false));
        // todo: specify current user
        // the second user will detect the first user's tray, will not pull up tray for incomming connection
        tray_exist = Some(crate::check_process("--tray", false, false));
    } else {
        let mut c = crate::ipc::connect(1000, "").await?;
        c.send(&Data::CheckProcess((
            Some(("--server".to_owned(), false, false)),
            None,
        )))
        .await?;
        if let Some(Data::CheckProcess((_, Some(exist)))) = c.next_timeout(1000).await? {
            server_exist = Some(exist);
        }
        c.send(&Data::CheckProcess((
            Some(("--tray".to_owned(), true, false)),
            None,
        )))
        .await?;
        if let Some(Data::CheckProcess((_, Some(exist)))) = c.next_timeout(1000).await? {
            tray_exist = Some(exist)
        }
    };
    log::info!(
        "env args: {:?}, --server exist: {:?}, --tray exist: {:?}",
        std::env::args(),
        server_exist,
        tray_exist
    );
    if server_exist == Some(true) && tray_exist == Some(false) {
        #[cfg(target_os = "linux")]
        hbb_common::allow_err!(crate::platform::check_autostart_config());
        if is_root {
            #[cfg(windows)]
            crate::platform::run_as_user(vec!["--tray"])?;
            #[cfg(target_os = "linux")]
            crate::platform::run_as_user(vec!["--tray"], None)?;
        } else {
            crate::run_me(vec!["--tray"])?;
        }
    }
    Ok(())
}

pub fn start_tray() {
    allow_err!(make_tray());
}

pub fn make_tray() -> hbb_common::ResultType<()> {
    // https://github.com/tauri-apps/tray-icon/blob/dev/examples/tao.rs
    use hbb_common::anyhow::Context;
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    use tray_icon::{
        menu::{Menu, MenuEvent, MenuItem},
        TrayEvent, TrayIconBuilder,
    };
    let icon;
    #[cfg(target_os = "macos")]
    {
        let mode = dark_light::detect();
        const LIGHT: &[u8] = include_bytes!("../res/mac-tray-light-x2.png");
        const DARK: &[u8] = include_bytes!("../res/mac-tray-dark-x2.png");
        icon = match mode {
            dark_light::Mode::Dark => LIGHT,
            _ => DARK,
        };
    }
    #[cfg(not(target_os = "macos"))]
    {
        icon = include_bytes!("../res/tray-icon.ico");
    }
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(icon)
            .context("Failed to open icon path")?
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    let icon = tray_icon::icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .context("Failed to open icon")?;

    let event_loop = EventLoopBuilder::new().build();

    let tray_menu = Menu::new();
    let quit_i = MenuItem::new(translate("Exit".to_owned()), true, None);
    let open_i = MenuItem::new(translate("Open".to_owned()), true, None);
    tray_menu.append_items(&[&open_i, &quit_i]);
    let tooltip = |count: usize| {
        if count == 0 {
            format!(
                "{} {}",
                crate::get_app_name(),
                translate("Service is running".to_owned()),
            )
        } else {
            format!(
                "{} - {} {}",
                crate::get_app_name(),
                count,
                translate("controlled sessions".to_owned())
            )
        }
    };
    let tray_icon = Some(
        TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip(tooltip(0))
            .with_icon(icon)
            .build()?,
    );
    let tray_icon = Arc::new(Mutex::new(tray_icon));

    let menu_channel = MenuEvent::receiver();
    let tray_channel = TrayEvent::receiver();
    #[cfg(not(target_os = "linux"))]
    let (ipc_sender, ipc_receiver) = std::sync::mpsc::channel::<Data>();
    let mut docker_hiden = false;

    let open_func = move || {
        if cfg!(not(feature = "flutter")) {
            crate::run_me::<&str>(vec![]).ok();
            return;
        }
        #[cfg(target_os = "macos")]
        crate::platform::macos::handle_application_should_open_untitled_file();
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            use std::process::Command;
            Command::new("cmd")
                .arg("/c")
                .arg("start rustdesk://")
                .creation_flags(winapi::um::winbase::CREATE_NO_WINDOW)
                .spawn()
                .ok();
        }
        #[cfg(target_os = "linux")]
        if !std::process::Command::new("xdg-open")
            .arg("rustdesk://")
            .spawn()
            .is_ok()
        {
            crate::run_me::<&str>(vec![]).ok();
        }
    };

    // ubuntu 22.04 can't see tooltip
    #[cfg(not(target_os = "linux"))]
    std::thread::spawn(move || {
        start_query_session_count(ipc_sender.clone());
    });
    event_loop.run(move |_event, _, control_flow| {
        if !docker_hiden {
            #[cfg(target_os = "macos")]
            crate::platform::macos::hide_dock();
            docker_hiden = true;
        }
        *control_flow = ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(100),
        );

        if let Ok(event) = menu_channel.try_recv() {
            if event.id == quit_i.id() {
                /* failed in windows, seems no permission to check system process
                if !crate::check_process("--server", false) {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                */
                if !crate::platform::uninstall_service(false) {
                    *control_flow = ControlFlow::Exit;
                }
            } else if event.id == open_i.id() {
                open_func();
            }
        }

        if let Ok(_event) = tray_channel.try_recv() {
            #[cfg(target_os = "windows")]
            if _event.event == tray_icon::ClickEvent::Left {
                open_func();
            }
        }

        #[cfg(not(target_os = "linux"))]
        if let Ok(data) = ipc_receiver.try_recv() {
            match data {
                Data::ControlledSessionCount(count) => {
                    tray_icon
                        .lock()
                        .unwrap()
                        .as_mut()
                        .map(|t| t.set_tooltip(Some(tooltip(count))));
                }
                _ => {}
            }
        }
    });
}

#[cfg(not(target_os = "linux"))]
#[tokio::main(flavor = "current_thread")]
async fn start_query_session_count(sender: std::sync::mpsc::Sender<Data>) {
    let mut last_count = 0;
    loop {
        if let Ok(mut c) = crate::ipc::connect(1000, "").await {
            let mut timer = tokio::time::interval(Duration::from_secs(1));
            loop {
                tokio::select! {
                    res = c.next() => {
                        match res {
                            Err(err) => {
                                log::error!("ipc connection closed: {}", err);
                                break;
                            }

                            Ok(Some(Data::ControlledSessionCount(count))) => {
                                if count != last_count {
                                    last_count = count;
                                    sender.send(Data::ControlledSessionCount(count)).ok();
                                }
                            }
                            _ => {}
                        }
                    }

                    _ = timer.tick() => {
                        c.send(&Data::ControlledSessionCount(0)).await.ok();
                    }
                }
            }
        }
        hbb_common::sleep(1.).await;
    }
}
