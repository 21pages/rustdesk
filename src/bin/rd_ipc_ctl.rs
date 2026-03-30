use clap::{Arg, ArgAction, Command};
use librustdesk::{common, ipc};
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

fn main() {
    if !common::global_init() {
        eprintln!("global_init failed");
        std::process::exit(1);
    }

    let matches = Command::new("rd_ipc_ctl")
        .about("Drive a running RustDesk desktop app through its IPC/url entrypoints")
        .subcommand(
            Command::new("connect")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .required(true)
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("password")
                        .long("password")
                        .required(false)
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("relay")
                        .long("relay")
                        .required(false)
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("connect-type")
                        .long("connect-type")
                        .required(false)
                        .default_value("remote")
                        .value_parser([
                            "remote",
                            "file-transfer",
                            "view-camera",
                            "terminal",
                            "rdp",
                            "port-forward",
                        ]),
                )
                .arg(
                    Arg::new("app-binary")
                        .long("app-binary")
                        .required(false)
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("scheme")
                        .long("scheme")
                        .required(false)
                        .default_value("rustdesk")
                        .action(ArgAction::Set),
                ),
        )
        .subcommand(Command::new("close-all"))
        .get_matches();

    let res = match matches.subcommand() {
        Some(("connect", sub)) => {
            let id = sub
                .get_one::<String>("id")
                .expect("required by clap")
                .to_owned();
            let password = sub.get_one::<String>("password").cloned();
            let relay = sub.get_flag("relay");
            let connect_type = sub
                .get_one::<String>("connect-type")
                .expect("defaulted by clap")
                .to_owned();
            let app_binary = sub.get_one::<String>("app-binary").map(PathBuf::from);
            let scheme = sub
                .get_one::<String>("scheme")
                .expect("defaulted by clap")
                .to_owned();
            connect(id, password, relay, connect_type, app_binary, scheme)
        }
        Some(("close-all", _)) => close_all(),
        _ => Err("missing subcommand".to_owned()),
    };

    if let Err(err) = res {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn connect(
    id: String,
    password: Option<String>,
    relay: bool,
    connect_type: String,
    app_binary: Option<PathBuf>,
    scheme: String,
) -> Result<(), String> {
    let command = match connect_type.as_str() {
        "remote" => "connection/new",
        "file-transfer" => "file-transfer",
        "view-camera" => "view-camera",
        "terminal" => "terminal",
        "rdp" => "rdp",
        "port-forward" => "port-forward",
        other => return Err(format!("unsupported connect type: {other}")),
    };

    let mut url = format!("{scheme}://{command}/{id}");
    let mut params = Vec::new();
    if let Some(password) = password {
        params.push(format!("password={password}"));
    }
    if relay {
        params.push("relay=1".to_owned());
    }
    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    match ipc::send_url_scheme(url.clone()) {
        Ok(()) => Ok(()),
        Err(err) => {
            if let Some(app_binary) = app_binary {
                ProcessCommand::new(&app_binary)
                    .arg(&url)
                    .spawn()
                    .map_err(|spawn_err| {
                        format!(
                            "failed to send ipc ({err}) and failed to spawn {:?}: {spawn_err}",
                            app_binary
                        )
                    })?;
                Ok(())
            } else {
                Err(format!(
                    "failed to send url via ipc: {err}. pass --app-binary if the app may not be running"
                ))
            }
        }
    }
}

fn close_all() -> Result<(), String> {
    ipc::close_all_instances()
        .map(|_| ())
        .map_err(|err| format!("failed to close RustDesk instances via ipc: {err}"))
}
