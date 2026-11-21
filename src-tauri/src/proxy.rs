use crate::certs::{CertManager, DynamicSniResolver};
use crate::models::{ProxyEntry, ProxyLogEntry, ProxyServerStatus};
use bytes::Bytes;
use chrono::Utc;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{HeaderName, HeaderValue, CONNECTION, HOST, LOCATION, UPGRADE};
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoServerBuilder;
use parking_lot::RwLock;
use rustls::ServerConfig;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio_rustls::TlsAcceptor;
use uuid::Uuid;

pub type LogCallback = Arc<dyn Fn(ProxyLogEntry) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyTarget {
    pub host: String,
    pub port: u16,
}

impl ProxyTarget {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

pub type RouteTable = Arc<RwLock<HashMap<String, ProxyTarget>>>;

pub fn build_route_map(proxies: &[ProxyEntry]) -> HashMap<String, ProxyTarget> {
    let mut map = HashMap::new();
    for p in proxies {
        if p.enabled {
            map.insert(
                p.hostname.to_ascii_lowercase(),
                ProxyTarget {
                    host: p.target_host.clone(),
                    port: p.target_port,
                },
            );
        }
    }
    map
}

pub fn is_port_available(port: u16) -> bool {
    StdTcpListener::bind(("127.0.0.1", port)).is_ok()
}

pub struct RunningProxy {
    shutdown_tx: watch::Sender<bool>,
    pub https_port: u16,
    pub http_port: Option<u16>,
}

impl RunningProxy {
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

pub struct ProxyController {
    routes: RouteTable,
    running: RwLock<Option<RunningProxy>>,
    last_error: RwLock<Option<String>>,
}

impl ProxyController {
    pub fn new(initial_proxies: &[ProxyEntry]) -> Self {
        Self {
            routes: Arc::new(RwLock::new(build_route_map(initial_proxies))),
            running: RwLock::new(None),
            last_error: RwLock::new(None),
        }
    }

    pub fn update_routes(&self, proxies: &[ProxyEntry]) {
        let mut guard = self.routes.write();
        *guard = build_route_map(proxies);
    }

    pub fn status(&self, default_https_port: u16) -> ProxyServerStatus {
        let running_guard = self.running.read();
        let active_routes = self.routes.read().len();
        let last_error = self.last_error.read().clone();

        match running_guard.as_ref() {
            Some(r) => ProxyServerStatus {
                running: true,
                https_port: r.https_port,
                http_port: r.http_port,
                active_routes,
                last_error,
            },
            None => ProxyServerStatus {
                running: false,
                https_port: default_https_port,
                http_port: None,
                active_routes,
                last_error,
            },
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.read().is_some()
    }

    pub fn stop(&self) {
        if let Some(r) = self.running.write().take() {
            r.stop();
        }
    }

    pub async fn start(
        &self,
        cert_manager: Arc<CertManager>,
        https_port: u16,
        http_port: Option<u16>,
        on_log: LogCallback,
    ) -> Result<(u16, Option<u16>), String> {
        // Stop any existing server first
        self.stop();

        // Ensure Root CA exists before accepting TLS handshakes
        cert_manager.ensure_root_ca()?;

        let mut tls_config = ServerConfig::builder_with_provider(
            rustls::crypto::ring::default_provider().into(),
        )
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("Failed to configure TLS protocol versions: {e}"))?
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(DynamicSniResolver::new(Arc::clone(&cert_manager))));
        tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

        let https_listener = TcpListener::bind(("127.0.0.1", https_port))
            .await
            .map_err(|e| {
                let msg = format!(
                    "Failed to bind HTTPS port {https_port}: {e}. Check if another service (e.g. Nginx, Caddy, Apache, IIS) is using port {https_port}, or change the HTTPS port in Settings."
                );
                *self.last_error.write() = Some(msg.clone());
                msg
            })?;
        let bound_https_port = https_listener
            .local_addr()
            .map(|a| a.port())
            .unwrap_or(https_port);

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Optional HTTP -> HTTPS redirect listener
        let mut bound_http_port = None;
        if let Some(hp) = http_port {
            match TcpListener::bind(("127.0.0.1", hp)).await {
                Ok(http_listener) => {
                    let actual_hp = http_listener.local_addr().map(|a| a.port()).unwrap_or(hp);
                    bound_http_port = Some(actual_hp);
                    spawn_http_redirect_loop(http_listener, bound_https_port, shutdown_rx.clone());
                }
                Err(e) => {
                    tracing::warn!("Could not bind HTTP redirect port {hp}: {e}");
                    *self.last_error.write() = Some(format!(
                        "HTTPS is running on :{bound_https_port}, but HTTP redirect port :{hp} could not be bound ({e})."
                    ));
                }
            }
        } else {
            *self.last_error.write() = None;
        }

        spawn_https_accept_loop(
            https_listener,
            tls_acceptor,
            Arc::clone(&self.routes),
            on_log,
            shutdown_rx,
        );

        *self.running.write() = Some(RunningProxy {
            shutdown_tx,
            https_port: bound_https_port,
            http_port: bound_http_port,
        });

        Ok((bound_https_port, bound_http_port))
    }
}

fn spawn_https_accept_loop(
    listener: TcpListener,
    acceptor: TlsAcceptor,
    routes: RouteTable,
    on_log: LogCallback,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                }
                accept_res = listener.accept() => {
                    let (stream, peer_addr) = match accept_res {
                        Ok(pair) => pair,
                        Err(e) => {
                            tracing::error!("HTTPS accept error: {e}");
                            continue;
                        }
                    };

                    let acceptor = acceptor.clone();
                    let routes = Arc::clone(&routes);
                    let on_log = Arc::clone(&on_log);

                    tokio::spawn(async move {
                        let tls_stream = match acceptor.accept(stream).await {
                            Ok(s) => s,
                            Err(e) => {
                                tracing::debug!("TLS handshake error from {peer_addr}: {e}");
                                return;
                            }
                        };

                        let io = TokioIo::new(tls_stream);
                        let service = service_fn(move |req: Request<Incoming>| {
                            let routes = Arc::clone(&routes);
                            let on_log = Arc::clone(&on_log);
                            async move {
                                Ok::<_, Infallible>(
                                    handle_proxy_request(req, peer_addr, routes, on_log).await,
                                )
                            }
                        });

                        let mut builder = AutoServerBuilder::new(TokioExecutor::new());
                        builder.http1().preserve_header_case(true);
                        if let Err(e) = builder.serve_connection_with_upgrades(io, service).await {
                            tracing::debug!("HTTPS connection error from {peer_addr}: {e}");
                        }
                    });
                }
            }
        }
    });
}

fn spawn_http_redirect_loop(
    listener: TcpListener,
    https_port: u16,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                }
                accept_res = listener.accept() => {
                    let (stream, _) = match accept_res {
                        Ok(pair) => pair,
                        Err(_) => continue,
                    };

                    tokio::spawn(async move {
                        let io = TokioIo::new(stream);
                        let service = service_fn(move |req: Request<Incoming>| async move {
                            let host_header = extract_host(&req).unwrap_or_else(|| "localhost".to_string());
                            let path_and_query = req
                                .uri()
                                .path_and_query()
                                .map(|pq| pq.as_str())
                                .unwrap_or("/");
                            let target_url = if https_port == 443 {
                                format!("https://{host_header}{path_and_query}")
                            } else {
                                format!("https://{host_header}:{https_port}{path_and_query}")
                            };

                            let resp = Response::builder()
                                .status(StatusCode::MOVED_PERMANENTLY)
                                .header(LOCATION, target_url)
                                .body(full_body(""))
                                .unwrap();
                            Ok::<_, Infallible>(resp)
                        });

                        let _ = hyper::server::conn::http1::Builder::new()
                            .serve_connection(io, service)
                            .await;
                    });
                }
            }
        }
    });
}

fn extract_host<B>(req: &Request<B>) -> Option<String> {
    if let Some(h) = req.headers().get(HOST).and_then(|v| v.to_str().ok()) {
        return Some(strip_port(h).to_ascii_lowercase());
    }
    req.uri().host().map(|h| h.to_ascii_lowercase())
}

fn strip_port(host: &str) -> &str {
    // Handle IPv6 bracketed hosts if ever used, otherwise split on ':'
    if host.starts_with('[') {
        host.split(']').next().unwrap_or(host)
    } else {
        host.split(':').next().unwrap_or(host)
    }
}

fn is_upgrade_request<B>(req: &Request<B>) -> bool {
    let has_upgrade_header = req.headers().contains_key(UPGRADE);
    let has_connection_upgrade = req
        .headers()
        .get(CONNECTION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.to_ascii_lowercase().contains("upgrade"));
    has_upgrade_header && has_connection_upgrade
}

fn full_body(content: impl Into<Bytes>) -> BoxBody<Bytes, hyper::Error> {
    Full::new(content.into())
        .map_err(|never| match never {})
        .boxed()
}

fn html_error_response(status: StatusCode, title: &str, detail: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>{code} {title} — LocalTLS</title>
  <style>
    body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: #09090b; color: #f4f4f5; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; }}
    .card {{ max-width: 520px; background: #18181b; border: 1px solid #27272a; border-radius: 12px; padding: 28px; box-shadow: 0 20px 50px rgba(0,0,0,0.4); }}
    .badge {{ display: inline-block; padding: 4px 10px; border-radius: 999px; font-size: 12px; font-weight: 600; background: #27272a; color: #f59e0b; margin-bottom: 12px; }}
    h1 {{ margin: 0 0 8px; font-size: 20px; }}
    p {{ margin: 0; color: #a1a1aa; line-height: 1.5; font-size: 14px; }}
    code {{ background: #27272a; color: #34d399; padding: 2px 6px; border-radius: 4px; font-size: 13px; }}
  </style>
</head>
<body>
  <div class="card">
    <div class="badge">LocalTLS • HTTP {code}</div>
    <h1>{title}</h1>
    <p>{detail}</p>
  </div>
</body>
</html>"#,
        code = status.as_u16()
    );

    Response::builder()
        .status(status)
        .header("content-type", "text/html; charset=utf-8")
        .body(full_body(html))
        .unwrap()
}

async fn handle_proxy_request(
    mut req: Request<Incoming>,
    peer_addr: SocketAddr,
    routes: RouteTable,
    on_log: LogCallback,
) -> Response<BoxBody<Bytes, hyper::Error>> {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| "/".to_string());
    let host = extract_host(&req).unwrap_or_else(|| "unknown".to_string());
    let is_ws = is_upgrade_request(&req);

    let target = { routes.read().get(&host).cloned() };
    let Some(target) = target else {
        let resp = html_error_response(
            StatusCode::NOT_FOUND,
            "Host Not Configured",
            &format!(
                "No enabled LocalTLS proxy rule matches <code>{host}</code>. Add or enable it in the LocalTLS desktop app."
            ),
        );
        on_log(ProxyLogEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            method,
            host,
            path,
            upstream: "unmatched".to_string(),
            status: 404,
            duration_ms: start.elapsed().as_millis() as u64,
            is_websocket: is_ws,
            error: Some("No matching active route".to_string()),
        });
        return resp;
    };

    let upstream_addr = target.addr();

    let upstream_stream = match TcpStream::connect(&upstream_addr).await {
        Ok(s) => s,
        Err(e) => {
            let err_msg = format!("Cannot connect to upstream {upstream_addr}: {e}");
            let resp = html_error_response(
                StatusCode::BAD_GATEWAY,
                "Upstream Server Unreachable",
                &format!(
                    "LocalTLS could not reach <code>{upstream_addr}</code> for <code>{host}</code>. Make sure your local dev server is running on port <code>{}</code>.",
                    target.port
                ),
            );
            on_log(ProxyLogEntry {
                id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                method,
                host,
                path,
                upstream: upstream_addr,
                status: 502,
                duration_ms: start.elapsed().as_millis() as u64,
                is_websocket: is_ws,
                error: Some(err_msg),
            });
            return resp;
        }
    };

    let io = TokioIo::new(upstream_stream);
    let (mut sender, conn) = match hyper::client::conn::http1::Builder::new()
        .preserve_header_case(true)
        .handshake(io)
        .await
    {
        Ok(pair) => pair,
        Err(e) => {
            let err_msg = format!("Upstream handshake failed: {e}");
            let resp = html_error_response(
                StatusCode::BAD_GATEWAY,
                "Upstream Handshake Failed",
                &err_msg,
            );
            on_log(ProxyLogEntry {
                id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                method,
                host,
                path,
                upstream: upstream_addr,
                status: 502,
                duration_ms: start.elapsed().as_millis() as u64,
                is_websocket: is_ws,
                error: Some(err_msg),
            });
            return resp;
        }
    };

    tokio::spawn(async move {
        if let Err(e) = conn.with_upgrades().await {
            tracing::debug!("Upstream connection closed: {e}");
        }
    });

    // Normalize URI to origin-form (required for HTTP/1.1 upstream requests when client sent HTTP/2)
    let origin_uri: hyper::Uri = path.parse().unwrap_or_else(|_| "/".parse().unwrap());
    *req.uri_mut() = origin_uri;
    *req.version_mut() = hyper::Version::HTTP_11;

    // Ensure Host header is present (HTTP/2 requests use :authority instead of Host)
    if !req.headers().contains_key(HOST) {
        if let Ok(hv) = HeaderValue::from_str(&host) {
            req.headers_mut().insert(HOST, hv);
        }
    }

    // Standard reverse-proxy headers
    req.headers_mut().insert(
        HeaderName::from_static("x-forwarded-proto"),
        HeaderValue::from_static("https"),
    );
    if let Ok(hv) = HeaderValue::from_str(&host) {
        req.headers_mut()
            .insert(HeaderName::from_static("x-forwarded-host"), hv);
    }
    if let Ok(hv) = HeaderValue::from_str(&peer_addr.ip().to_string()) {
        req.headers_mut()
            .insert(HeaderName::from_static("x-forwarded-for"), hv);
    }

    // Capture client upgrade future before moving req into sender
    let client_upgrade = if is_ws {
        Some(hyper::upgrade::on(&mut req))
    } else {
        None
    };

    let upstream_resp = match sender.send_request(req).await {
        Ok(res) => res,
        Err(e) => {
            let err_msg = format!("Upstream request error: {e}");
            let resp = html_error_response(
                StatusCode::BAD_GATEWAY,
                "Upstream Request Failed",
                &err_msg,
            );
            on_log(ProxyLogEntry {
                id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                method,
                host,
                path,
                upstream: upstream_addr,
                status: 502,
                duration_ms: start.elapsed().as_millis() as u64,
                is_websocket: is_ws,
                error: Some(err_msg),
            });
            return resp;
        }
    };

    let status_code = upstream_resp.status();

    if status_code == StatusCode::SWITCHING_PROTOCOLS {
        let mut resp_builder = Response::builder().status(status_code);
        for (k, v) in upstream_resp.headers() {
            resp_builder = resp_builder.header(k, v);
        }
        let upstream_upgrade = hyper::upgrade::on(upstream_resp);

        if let Some(client_upg) = client_upgrade {
            tokio::spawn(async move {
                match tokio::try_join!(client_upg, upstream_upgrade) {
                    Ok((c_io, u_io)) => {
                        let mut c_stream = TokioIo::new(c_io);
                        let mut u_stream = TokioIo::new(u_io);
                        let _ = copy_bidirectional(&mut c_stream, &mut u_stream).await;
                    }
                    Err(e) => {
                        tracing::debug!("WebSocket upgrade tunnel error: {e}");
                    }
                }
            });
        }

        on_log(ProxyLogEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            method: "WS".to_string(),
            host,
            path,
            upstream: upstream_addr,
            status: 101,
            duration_ms: start.elapsed().as_millis() as u64,
            is_websocket: true,
            error: None,
        });

        return resp_builder.body(full_body("")).unwrap();
    }

    let (parts, body) = upstream_resp.into_parts();
    let boxed_body: BoxBody<Bytes, hyper::Error> = body.boxed();

    on_log(ProxyLogEntry {
        id: Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        method,
        host,
        path,
        upstream: upstream_addr,
        status: status_code.as_u16(),
        duration_ms: start.elapsed().as_millis() as u64,
        is_websocket: false,
        error: None,
    });

    Response::from_parts(parts, boxed_body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use rustls::pki_types::ServerName;
    use rustls::{ClientConfig, RootCertStore};
    use std::sync::Mutex;
    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_rustls::TlsConnector;
    use tokio_tungstenite::tungstenite::Message;

    fn make_tls_connector(ca_pem: &str) -> TlsConnector {
        let mut root_store = RootCertStore::empty();
        let mut reader = std::io::BufReader::new(ca_pem.as_bytes());
        for cert in rustls_pemfile::certs(&mut reader) {
            root_store.add(cert.unwrap()).unwrap();
        }
        let cfg = ClientConfig::builder_with_provider(
            rustls::crypto::ring::default_provider().into(),
        )
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(root_store)
        .with_no_client_auth();
        TlsConnector::from(Arc::new(cfg))
    }

    async fn spawn_mock_http_server(label: &'static str) -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);
                    let svc = service_fn(move |req: Request<Incoming>| async move {
                        let proto = req
                            .headers()
                            .get("x-forwarded-proto")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("missing");
                        let body = format!("{label}|{}|{proto}", req.uri().path());
                        Ok::<_, Infallible>(Response::new(Full::new(Bytes::from(body))))
                    });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, svc)
                        .await;
                });
            }
        });
        port
    }

    async fn spawn_mock_ws_server() -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    if let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await {
                        while let Some(Ok(msg)) = ws.next().await {
                            if msg.is_text() {
                                let _ = ws.send(Message::Text(format!("echo:{}", msg.to_text().unwrap()).into())).await;
                            }
                        }
                    }
                });
            }
        });
        port
    }

    async fn https_get(
        connector: &TlsConnector,
        port: u16,
        hostname: &str,
        path: &str,
    ) -> String {
        let tcp = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let server_name = ServerName::try_from(hostname.to_string()).unwrap();
        let mut tls = connector.connect(server_name, tcp).await.unwrap();
        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: {hostname}\r\nConnection: close\r\n\r\n"
        );
        tls.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        tls.read_to_end(&mut buf).await.unwrap();
        String::from_utf8_lossy(&buf).to_string()
    }

    #[tokio::test]
    async fn test_end_to_end_https_sni_hot_reload_redirect_and_websocket() {
        let dir = TempDir::new().unwrap();
        let cert_mgr = Arc::new(CertManager::new(dir.path().to_path_buf()));
        cert_mgr.ensure_root_ca().unwrap();
        let ca_pem = cert_mgr.ca_cert_pem().unwrap();
        let connector = make_tls_connector(&ca_pem);

        let upstream_a = spawn_mock_http_server("service-a").await;
        let upstream_b = spawn_mock_http_server("service-b").await;
        let upstream_ws = spawn_mock_ws_server().await;

        let initial_proxies = vec![
            ProxyEntry {
                id: "1".into(),
                hostname: "a.localtls.test".into(),
                target_host: "127.0.0.1".into(),
                target_port: upstream_a,
                enabled: true,
                group_id: "default".into(),
                created_at: Utc::now(),
                tunnel_enabled: false,
                tunnel_route_name: None,
            },
            ProxyEntry {
                id: "2".into(),
                hostname: "ws.localtls.test".into(),
                target_host: "127.0.0.1".into(),
                target_port: upstream_ws,
                enabled: true,
                group_id: "default".into(),
                created_at: Utc::now(),
                tunnel_enabled: false,
                tunnel_route_name: None,
            },
        ];

        let logs: Arc<Mutex<Vec<ProxyLogEntry>>> = Arc::new(Mutex::new(Vec::new()));
        let logs_cb = Arc::clone(&logs);
        let controller = ProxyController::new(&initial_proxies);

        let (https_port, http_port) = controller
            .start(
                Arc::clone(&cert_mgr),
                0,
                Some(0),
                Arc::new(move |entry| {
                    logs_cb.lock().unwrap().push(entry);
                }),
            )
            .await
            .unwrap();

        // 1. HTTPS GET to service-a verified against our Root CA
        let res_a = https_get(&connector, https_port, "a.localtls.test", "/hello").await;
        assert!(res_a.contains("200 OK"), "unexpected response: {res_a}");
        assert!(res_a.contains("service-a|/hello|https"));

        // 2. Unknown hostname returns 404
        let res_404 = https_get(&connector, https_port, "unknown.localtls.test", "/").await;
        assert!(res_404.contains("404 Not Found"));

        // 3. Hot-reload routes without restarting server
        let mut updated_proxies = initial_proxies.clone();
        updated_proxies.push(ProxyEntry {
            id: "3".into(),
            hostname: "b.localtls.test".into(),
            target_host: "127.0.0.1".into(),
            target_port: upstream_b,
            enabled: true,
            group_id: "default".into(),
            created_at: Utc::now(),
            tunnel_enabled: false,
            tunnel_route_name: None,
        });
        controller.update_routes(&updated_proxies);

        let res_b = https_get(&connector, https_port, "b.localtls.test", "/world").await;
        assert!(res_b.contains("200 OK"));
        assert!(res_b.contains("service-b|/world|https"));

        // 4. HTTP -> HTTPS 301 redirect
        let hp = http_port.unwrap();
        let mut http_tcp = TcpStream::connect(("127.0.0.1", hp)).await.unwrap();
        http_tcp
            .write_all(b"GET /login?x=1 HTTP/1.1\r\nHost: a.localtls.test\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut http_buf = Vec::new();
        http_tcp.read_to_end(&mut http_buf).await.unwrap();
        let http_resp = String::from_utf8_lossy(&http_buf);
        assert!(http_resp.contains("301 Moved Permanently"));
        assert!(http_resp.contains(&format!("https://a.localtls.test:{https_port}/login?x=1")));

        // 5. WebSocket over TLS (wss://)
        let ws_tcp = TcpStream::connect(("127.0.0.1", https_port)).await.unwrap();
        let ws_tls = connector
            .connect(
                ServerName::try_from("ws.localtls.test".to_string()).unwrap(),
                ws_tcp,
            )
            .await
            .unwrap();
        let (mut ws_stream, _) =
            tokio_tungstenite::client_async("wss://ws.localtls.test/socket", ws_tls)
                .await
                .unwrap();
        ws_stream.send(Message::Text("ping-hmr".into())).await.unwrap();
        let reply = ws_stream.next().await.unwrap().unwrap();
        assert_eq!(reply.to_text().unwrap(), "echo:ping-hmr");

        // 6. Logs were captured
        let captured = logs.lock().unwrap();
        assert!(captured.len() >= 4);
        assert!(captured.iter().any(|l| l.is_websocket && l.status == 101));

        controller.stop();
    }
}
