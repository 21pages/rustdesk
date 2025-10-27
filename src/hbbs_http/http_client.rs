use hbb_common::config::Config;
use hbb_common::flexi_logger::Cleanup;
use hbb_common::log::info;
use hbb_common::proxy::{Proxy, ProxyScheme};
use hbb_common::rustls_platform_verifier;
use hbb_common::ResultType;
use reqwest::blocking::Client as SyncClient;
use reqwest::Client as AsyncClient;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TlsMethod {
    PlatformVerifier,
    Default,
}

lazy_static::lazy_static! {
    static ref TLS_METHOD_CACHE: Mutex<HashMap<String, TlsMethod>> = Mutex::new(HashMap::new());
}

/// Generate cache key from URL (format: "host:port")
fn get_cache_key(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    let port = parsed.port_or_known_default()?;
    Some(format!("{}:{}", host, port))
}

macro_rules! apply_proxy_config {
    ($builder:expr, $Client: ty) => {{
        // https://github.com/rustdesk/rustdesk/issues/11569
        // https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html#method.no_proxy
        let mut builder = $builder.no_proxy();
        if let Some(conf) = Config::get_socks() {
            let proxy_result = Proxy::from_conf(&conf, None);

            match proxy_result {
                Ok(proxy) => {
                    let proxy_setup = match &proxy.intercept {
                        ProxyScheme::Http { host, .. } => {
                            reqwest::Proxy::all(format!("http://{}", host))
                        }
                        ProxyScheme::Https { host, .. } => {
                            reqwest::Proxy::all(format!("https://{}", host))
                        }
                        ProxyScheme::Socks5 { addr, .. } => {
                            reqwest::Proxy::all(&format!("socks5://{}", addr))
                        }
                    };

                    match proxy_setup {
                        Ok(p) => {
                            builder = builder.proxy(p);
                            if let Some(auth) = proxy.intercept.maybe_auth() {
                                let basic_auth =
                                    format!("Basic {}", auth.get_basic_authorization());
                                if let Ok(auth) = basic_auth.parse() {
                                    builder = builder.default_headers(
                                        vec![(reqwest::header::PROXY_AUTHORIZATION, auth)]
                                            .into_iter()
                                            .collect(),
                                    );
                                }
                            }
                            builder.build().unwrap_or_else(|e| {
                                info!("Failed to create a proxied client: {}", e);
                                <$Client>::new()
                            })
                        }
                        Err(e) => {
                            info!("Failed to set up proxy: {}", e);
                            <$Client>::new()
                        }
                    }
                }
                Err(e) => {
                    info!("Failed to configure proxy: {}", e);
                    <$Client>::new()
                }
            }
        } else {
            builder.build().unwrap_or_else(|e| {
                info!("Failed to create a client: {}", e);
                <$Client>::new()
            })
        }
    }};
}

/// Create a blocking HTTP client with proxy configuration
pub fn create_default_http_client() -> SyncClient {
    let builder = SyncClient::builder();
    apply_proxy_config!(builder, SyncClient)
}

/// Create a blocking HTTP client with URL-based fallback mechanism
///
/// Similar to async version, but performs synchronous TLS testing.
/// For HTTPS URLs:
/// 1. Checks cache first
/// 2. If no cache, tests platform_verifier
/// 3. Falls back to default if platform_verifier fails
pub fn create_http_client(url: &str) -> SyncClient {
    // Parse URL
    let parsed = match reqwest::Url::parse(url) {
        Ok(p) => p,
        Err(_) => return create_default_http_client(),
    };

    // For non-HTTPS, use default client with proxy config
    if parsed.scheme() != "https" {
        return create_default_http_client();
    }

    // For HTTPS, check cache and test TLS methods
    let cache_key = match get_cache_key(url) {
        Some(k) => k,
        None => return create_default_http_client(),
    };

    // Check cache
    if let Ok(cache) = TLS_METHOD_CACHE.lock() {
        if let Some(&method) = cache.get(&cache_key) {
            info!("Using cached TLS method {:?} for {}", method, cache_key);
            return create_sync_client_with_method(method);
        }
    }

    // No cache, try platform_verifier first
    if let Ok(client) = test_sync_tls_connection(url) {
        info!("TLS with platform_verifier succeeded for {}", cache_key);
        // Cache the successful method
        if let Ok(mut cache) = TLS_METHOD_CACHE.lock() {
            cache.insert(cache_key, TlsMethod::PlatformVerifier);
        }
        return client;
    }

    // platform_verifier failed, fall back to default
    info!(
        "TLS with platform_verifier failed for {}, falling back to default",
        cache_key
    );
    if let Ok(mut cache) = TLS_METHOD_CACHE.lock() {
        cache.insert(cache_key, TlsMethod::Default);
    }

    create_sync_client_with_method(TlsMethod::Default)
}

fn create_sync_client_with_method(method: TlsMethod) -> SyncClient {
    use hbb_common::rustls_platform_verifier::ConfigVerifierExt;
    use hbb_common::tokio_rustls::rustls::ClientConfig;

    let builder = SyncClient::builder();

    // Apply TLS configuration
    let builder = match method {
        TlsMethod::PlatformVerifier => match ClientConfig::with_platform_verifier() {
            Ok(tls) => builder.use_preconfigured_tls(tls),
            Err(_) => builder,
        },
        TlsMethod::Default => builder,
    };

    // Apply proxy configuration and build
    apply_proxy_config!(builder, SyncClient)
}

fn test_sync_tls_connection(url: &str) -> Result<SyncClient, ()> {
    use hbb_common::rustls_platform_verifier::ConfigVerifierExt;
    use hbb_common::tokio_rustls::rustls::ClientConfig;
    use std::net::TcpStream;
    use std::time::Duration;

    let timeout = Duration::from_secs(5);

    // Parse URL
    let parsed = reqwest::Url::parse(url).map_err(|_| ())?;
    let host = parsed.host_str().ok_or(())?;
    let port = parsed.port_or_known_default().ok_or(())?;

    // Establish TCP connection with timeout
    let addr = format!("{}:{}", host, port);
    TcpStream::connect_timeout(&addr.parse().map_err(|_| ())?, timeout).map_err(|_| ())?;

    // Try to create client with platform_verifier
    let config = ClientConfig::with_platform_verifier().map_err(|_| ())?;
    let builder = SyncClient::builder().use_preconfigured_tls(config);

    // Apply proxy configuration and build
    Ok(apply_proxy_config!(builder, SyncClient))
}

pub async fn create_http_client_async(url: &str) -> ResultType<AsyncClient> {
    use hbb_common::anyhow;

    // Parse URL
    let parsed = reqwest::Url::parse(url).map_err(|e| anyhow::anyhow!(e))?;

    // For non-HTTPS, use default client with proxy config
    if parsed.scheme() != "https" {
        let builder = AsyncClient::builder();
        return Ok(apply_proxy_config!(builder, AsyncClient));
    }

    // For HTTPS, check cache and test TLS methods
    let cache_key = get_cache_key(url).ok_or_else(|| anyhow::anyhow!("Invalid URL"))?;

    // Check cache
    if let Ok(cache) = TLS_METHOD_CACHE.lock() {
        if let Some(&method) = cache.get(&cache_key) {
            info!("Using cached TLS method {:?} for {}", method, cache_key);
            return create_async_client_with_method(method);
        }
    }

    // No cache, try platform_verifier first
    if let Ok(client) = test_tls_connection(url).await {
        info!("TLS with platform_verifier succeeded for {}", cache_key);
        // Cache the successful method
        if let Ok(mut cache) = TLS_METHOD_CACHE.lock() {
            cache.insert(cache_key, TlsMethod::PlatformVerifier);
        }
        return Ok(client);
    }

    // platform_verifier failed, fall back to default
    info!(
        "TLS with platform_verifier failed for {}, falling back to default",
        cache_key
    );
    if let Ok(mut cache) = TLS_METHOD_CACHE.lock() {
        cache.insert(cache_key, TlsMethod::Default);
    }

    create_async_client_with_method(TlsMethod::Default)
}

fn create_async_client_with_method(method: TlsMethod) -> ResultType<AsyncClient> {
    use hbb_common::anyhow;
    use hbb_common::rustls_platform_verifier::ConfigVerifierExt;
    use hbb_common::tokio_rustls::rustls::ClientConfig;

    let builder = AsyncClient::builder();

    // Apply TLS configuration
    let builder = match method {
        TlsMethod::PlatformVerifier => {
            let tls = ClientConfig::with_platform_verifier()?;
            builder.use_preconfigured_tls(tls)
        }
        TlsMethod::Default => builder,
    };

    // Apply proxy configuration and build
    Ok(apply_proxy_config!(builder, AsyncClient))
}

async fn test_tls_connection(url: &str) -> ResultType<AsyncClient> {
    use hbb_common::anyhow;
    use hbb_common::rustls_platform_verifier::ConfigVerifierExt;
    use hbb_common::tokio::net::TcpStream;
    use hbb_common::tokio_rustls::{rustls::ClientConfig, TlsConnector};
    use std::sync::Arc;
    use std::time::Duration;

    let timeout = Duration::from_secs(5);

    // Parse URL
    let parsed = reqwest::Url::parse(url).map_err(|e| anyhow::anyhow!(e))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("No host in URL"))?;
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| anyhow::anyhow!("No port in URL"))?;

    // Establish TCP connection
    let addr = format!("{}:{}", host, port);
    let stream = hbb_common::tokio::time::timeout(timeout, TcpStream::connect(&addr))
        .await
        .map_err(|_| anyhow::anyhow!("TCP connection timeout"))?
        .map_err(|e| anyhow::anyhow!("TCP connection failed: {}", e))?;

    // Test TLS handshake with platform_verifier
    let config = ClientConfig::with_platform_verifier()?;

    let connector = TlsConnector::from(Arc::new(config.clone()));
    let domain =
        hbb_common::tokio_rustls::rustls::pki_types::ServerName::try_from(host.to_string())
            .map_err(|e| anyhow::anyhow!("Invalid DNS name: {}", e))?;

    // Perform TLS handshake
    hbb_common::tokio::time::timeout(timeout, connector.connect(domain, stream))
        .await
        .map_err(|_| anyhow::anyhow!("TLS handshake timeout"))?
        .map_err(|e| anyhow::anyhow!("TLS handshake failed: {}", e))?;

    // TLS handshake succeeded, return client with the same config and proxy settings
    let builder = AsyncClient::builder().use_preconfigured_tls(config);
    Ok(apply_proxy_config!(builder, AsyncClient))
}
