use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

#[cfg(not(any(target_os = "ios")))]
use crate::Connection;
use hbb_common::{
    config::{Config, LocalConfig},
    tokio::{self, sync::broadcast, time::Instant},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const TIME_HEARTBEAT: Duration = Duration::from_secs(15);
const UPLOAD_SYSINFO_TIMEOUT: Duration = Duration::from_secs(120);
const TIME_CONN: Duration = Duration::from_secs(3);

#[cfg(not(any(target_os = "ios")))]
lazy_static::lazy_static! {
    static ref SENDER : Mutex<broadcast::Sender<Vec<i32>>> = Mutex::new(start_hbbs_sync());
    static ref PRO: Arc<Mutex<bool>> = Default::default();
}

#[cfg(not(any(target_os = "ios")))]
pub fn start() {
    let _sender = SENDER.lock().unwrap();
}

#[cfg(not(target_os = "ios"))]
pub fn signal_receiver() -> broadcast::Receiver<Vec<i32>> {
    SENDER.lock().unwrap().subscribe()
}

#[cfg(not(any(target_os = "ios")))]
fn start_hbbs_sync() -> broadcast::Sender<Vec<i32>> {
    let (tx, _rx) = broadcast::channel::<Vec<i32>>(16);
    std::thread::spawn(move || start_hbbs_sync_async());
    return tx;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StrategyOptions {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub config_options: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Default)]
struct SysinfoCtl {
    uploaded: bool,
    url: String,
    instant: Option<Instant>,
    id: String,
    username: String,
}

#[cfg(not(any(target_os = "ios")))]
#[tokio::main(flavor = "current_thread")]
async fn start_hbbs_sync_async() {
    let mut interval = crate::rustdesk_interval(tokio::time::interval_at(
        Instant::now() + TIME_CONN,
        TIME_CONN,
    ));
    let mut last_sent: Option<Instant> = None;
    let mut sysinfo_ctl = SysinfoCtl::default();
    let has_username = cfg!(not(any(target_os = "android", target_os = "ios")));
    let get_username = || {
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            let username = crate::platform::get_active_username();
            if !username.is_empty() && (!cfg!(windows) || username != "SYSTEM") {
                return username;
            }
        }
        String::default()
    };
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let url = heartbeat_url();
                let id = Config::get_id();
                if url.is_empty() {
                    *PRO.lock().unwrap() = false;
                    continue;
                }
                if !Config::get_option("stop-service").is_empty() {
                    continue;
                }
                let conns = Connection::alive_conns();
                if sysinfo_ctl.uploaded && (url != sysinfo_ctl.url || id != sysinfo_ctl.id){
                    sysinfo_ctl.uploaded = false;
                    *PRO.lock().unwrap() = false;
                }
                // username is empty in login screen
                let need_reupload_username = has_username && sysinfo_ctl.uploaded && sysinfo_ctl.username.is_empty() && !get_username().is_empty();
                if (!sysinfo_ctl.uploaded || need_reupload_username) && sysinfo_ctl.instant.map(|x| x.elapsed() >= UPLOAD_SYSINFO_TIMEOUT).unwrap_or(true) {
                    let mut v = crate::get_sysinfo();
                    let username = get_username();
                    if has_username {
                        v["username"] = json!(username.clone());
                    }
                    v["version"] = json!(crate::VERSION);
                    v["id"] = json!(id);
                    v["uuid"] = json!(crate::encode64(hbb_common::get_uuid()));
                    let ab_name = Config::get_option("preset-address-book-name");
                    if !ab_name.is_empty() {
                        v["preset-address-book-name"] = json!(ab_name);
                    }
                    let ab_tag = Config::get_option("preset-address-book-tag");
                    if !ab_tag.is_empty() {
                        v["preset-address-book-tag"] = json!(ab_tag);
                    }
                    match crate::post_request(url.replace("heartbeat", "sysinfo"), v.to_string(), "").await {
                        Ok(x)  => {
                            if x == "SYSINFO_UPDATED" {
                                sysinfo_ctl = SysinfoCtl {
                                    uploaded: true,
                                    url: url.clone(),
                                    instant: None,
                                    id: id.clone(),
                                    username,
                                };
                                *PRO.lock().unwrap() = true;
                            } else if x == "ID_NOT_FOUND" {
                                sysinfo_ctl.instant = None; // next heartbeat will upload sysinfo again
                            } else {
                                sysinfo_ctl.instant = Some(Instant::now());
                            }
                        }
                        _ => {
                            sysinfo_ctl.instant = Some(Instant::now());
                        }
                    }
                }
                if conns.is_empty() && last_sent.map(|x| x.elapsed() < TIME_HEARTBEAT).unwrap_or(false){
                    continue;
                }
                last_sent = Some(Instant::now());
                let mut v = Value::default();
                v["id"] = json!(id);
                v["uuid"] = json!(crate::encode64(hbb_common::get_uuid()));
                v["ver"] = json!(hbb_common::get_version_number(crate::VERSION));
                if !conns.is_empty() {
                    v["conns"] = json!(conns);
                }
                let modified_at = LocalConfig::get_option("strategy_timestamp").parse::<i64>().unwrap_or(0);
                v["modified_at"] = json!(modified_at);
                if let Ok(s) = crate::post_request(url.clone(), v.to_string(), "").await {
                    if let Ok(mut rsp) = serde_json::from_str::<HashMap::<&str, Value>>(&s) {
                        if let Some(conns)  = rsp.remove("disconnect") {
                                if let Ok(conns) = serde_json::from_value::<Vec<i32>>(conns) {
                                    SENDER.lock().unwrap().send(conns).ok();
                                }
                        }
                        if let Some(rsp_modified_at) = rsp.remove("modified_at") {
                            if let Ok(rsp_modified_at) = serde_json::from_value::<i64>(rsp_modified_at) {
                                if rsp_modified_at != modified_at {
                                    LocalConfig::set_option("strategy_timestamp".to_string(), rsp_modified_at.to_string());
                                }
                            }
                        }
                        if let Some(strategy) = rsp.remove("strategy") {
                            if let Ok(strategy) = serde_json::from_value::<StrategyOptions>(strategy) {
                                handle_config_options(strategy.config_options);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn heartbeat_url() -> String {
    let url = crate::common::get_api_server(
        Config::get_option("api-server"),
        Config::get_option("custom-rendezvous-server"),
    );
    if url.is_empty() || url.contains("rustdesk.com") {
        return "".to_owned();
    }
    format!("{}/api/heartbeat", url)
}

fn handle_config_options(config_options: HashMap<String, String>) {
    let mut options = Config::get_options();
    config_options
        .iter()
        .map(|(k, v)| {
            if v.is_empty() {
                options.remove(k);
            } else {
                options.insert(k.to_string(), v.to_string());
            }
        })
        .count();
    Config::set_options(options);
}

#[allow(unused)]
#[cfg(not(any(target_os = "ios")))]
pub fn is_pro() -> bool {
    PRO.lock().unwrap().clone()
}
