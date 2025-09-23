use crate::common::post_request;

use super::create_http_client;
use hbb_common::{anyhow::anyhow, log, ResultType};
use serde_derive::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct DeployState {
    pub team: String,
    pub group: String,
    pub user: String,
}

impl DeployState {
    pub fn is_deployed(&self) -> bool {
        !self.team.is_empty()
    }
}

pub async fn check_deploy() -> ResultType<DeployState> {
    let api_server = crate::ui_interface::get_api_server();
    if !crate::is_public(&api_server) {
        return Err(anyhow!("API server is not public"));
    }
    let url = format!("{}/api/deploy/state", api_server);

    let body = json!({
        "id": hbb_common::config::Config::get_id(),
        "uuid": crate::ui_interface::get_uuid()
    });
    // let response = client
    //     .post(&url)
    //     .header("Content-Type", "application/json")
    //     .json(&body)
    //     .send()?;
    let response = post_request(url, body.to_string(), "").await?;

    // if !response.status().is_success() {
    //     return Err(anyhow!("HTTP {}", response.status()));
    // }

    // let json_response: serde_json::Value = response.json()?;
    let json_response: serde_json::Value = serde_json::from_str(&response)?;

    if let Some(error) = json_response.get("error") {
        if let Some(err_str) = error.as_str() {
            return Err(anyhow!("{}", err_str));
        }
    }

    let deploy_state = DeployState {
        team: json_response
            .get("team")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        group: json_response
            .get("group")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        user: json_response
            .get("user")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    };

    log::info!(
        "Deploy state checked: deployed={}, team={}",
        deploy_state.is_deployed(),
        deploy_state.team
    );
    Ok(deploy_state)
}
