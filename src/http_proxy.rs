use hbb_common::{
    bytes::Bytes,
    config::{Config, CONNECT_TIMEOUT},
    log,
    protobuf::Message as _,
    rendezvous_proto::{rendezvous_message, HttpProxyRequest, HttpProxyResponse, RendezvousMessage},
    socket_client::connect_tcp,
    ResultType,
};
use std::collections::HashMap;

const HTTP_PROXY_TIMEOUT: u64 = 30_000;

/// Send HTTP request via TCP connection to rendezvous server.
/// The server will proxy the request to the target URL.
///
/// # Arguments
/// * `method` - HTTP method (GET, POST, PUT, DELETE, etc.)
/// * `path` - The relative path (e.g., "/api/users")
/// * `headers` - HTTP headers
/// * `body` - Request body
///
/// # Returns
/// * `HttpProxyResponse` containing status, headers, body, and error
pub async fn http_request_via_tcp(
    method: &str,
    path: &str,
    headers: HashMap<String, String>,
    body: Vec<u8>,
) -> ResultType<HttpProxyResponse> {
    let server = Config::get_rendezvous_server();
    if server.is_empty() {
        hbb_common::bail!("Rendezvous server is not configured");
    }

    let server = crate::check_port(&server, hbb_common::config::RENDEZVOUS_PORT);
    log::debug!("HTTP proxy: connecting to {}", server);

    let mut conn = connect_tcp(server, CONNECT_TIMEOUT).await?;

    // Secure the connection
    let key = crate::get_key(true).await;
    crate::secure_tcp(&mut conn, &key).await?;

    // Build and send HttpProxyRequest
    let mut msg_out = RendezvousMessage::new();
    msg_out.set_http_proxy_request(HttpProxyRequest {
        method: method.to_string(),
        path: path.to_string(),
        headers,
        body: Bytes::from(body),
        ..Default::default()
    });

    conn.send(&msg_out).await?;
    log::debug!("HTTP proxy: request sent, waiting for response");

    // Wait for response
    if let Some(msg_in) = crate::get_next_nonkeyexchange_msg(&mut conn, Some(HTTP_PROXY_TIMEOUT)).await {
        if let Some(rendezvous_message::Union::HttpProxyResponse(response)) = msg_in.union {
            log::debug!(
                "HTTP proxy: received response, status={}, error={}",
                response.status,
                response.error
            );
            return Ok(response);
        }
    }

    hbb_common::bail!("Failed to receive HTTP proxy response")
}

/// POST request via TCP proxy
pub async fn post_via_tcp(
    path: &str,
    body: String,
    content_type: &str,
) -> ResultType<String> {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), content_type.to_string());

    let response = http_request_via_tcp("POST", path, headers, body.into_bytes()).await?;

    if !response.error.is_empty() {
        hbb_common::bail!("{}", response.error);
    }

    if response.status < 200 || response.status >= 300 {
        hbb_common::bail!("HTTP error: status {}", response.status);
    }

    Ok(String::from_utf8_lossy(&response.body).to_string())
}

/// GET request via TCP proxy
pub async fn get_via_tcp(path: &str) -> ResultType<String> {
    let response = http_request_via_tcp("GET", path, HashMap::new(), vec![]).await?;

    if !response.error.is_empty() {
        hbb_common::bail!("{}", response.error);
    }

    if response.status < 200 || response.status >= 300 {
        hbb_common::bail!("HTTP error: status {}", response.status);
    }

    Ok(String::from_utf8_lossy(&response.body).to_string())
}

/// Check if HTTP proxy via TCP is enabled
pub fn is_http_proxy_enabled() -> bool {
    Config::get_option("enable-http-proxy") == "Y"
}
