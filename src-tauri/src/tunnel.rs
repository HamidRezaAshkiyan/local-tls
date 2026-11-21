use crate::models::{EditionInfo, ProxyEntry, TunnelStatusDto};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub type TunnelStatusCallback = Arc<dyn Fn(TunnelStatusDto) + Send + Sync>;

pub fn edition_info() -> EditionInfo {
    #[cfg(feature = "tunnel")]
    {
        EditionInfo {
            edition: "tunnel".to_string(),
            tunnel_supported: true,
        }
    }
    #[cfg(not(feature = "tunnel"))]
    {
        EditionInfo {
            edition: "standard".to_string(),
            tunnel_supported: false,
        }
    }
}

/// Derives a valid OpenTunnel route label (lowercase alphanumeric + hyphens, 1..=63 chars)
/// for a given proxy entry.
pub fn derive_route_name(proxy: &ProxyEntry) -> String {
    let raw = proxy
        .tunnel_route_name
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| proxy.hostname.split('.').next().unwrap_or("app"));

    let mut sanitized: String = raw
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();

    sanitized = sanitized.trim_matches('-').to_string();
    if sanitized.len() > 48 {
        sanitized.truncate(48);
        sanitized = sanitized.trim_end_matches('-').to_string();
    }

    if sanitized.is_empty() {
        let short_id: String = proxy
            .id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(8)
            .collect();
        if short_id.is_empty() {
            "app".to_string()
        } else {
            format!("app-{}", short_id.to_ascii_lowercase())
        }
    } else {
        sanitized
    }
}

/// Builds a map of `proxy.id -> route_name` for all proxies with `tunnel_enabled == true`,
/// disambiguating any duplicate route names deterministically.
pub fn compute_proxy_route_names(proxies: &[ProxyEntry]) -> HashMap<String, String> {
    let mut used = std::collections::HashSet::new();
    let mut out = HashMap::new();

    for proxy in proxies {
        if !proxy.tunnel_enabled {
            continue;
        }
        let base = derive_route_name(proxy);
        let mut candidate = base.clone();
        let mut suffix = 2usize;
        while used.contains(&candidate) {
            candidate = format!("{base}-{suffix}");
            suffix += 1;
        }
        used.insert(candidate.clone());
        out.insert(proxy.id.clone(), candidate);
    }

    out
}

#[cfg(feature = "tunnel")]
mod imp {
    use super::*;
    use opentunnel::{Client, ClientOptions, Route, Routes, State, Storage, Tunnel};
    use parking_lot::RwLock;
    use tokio::sync::Mutex as AsyncMutex;

    const PROFILE_NAME: &str = "localtls";

    struct ActiveTunnel {
        tunnel: Tunnel,
        hostname: String,
    }

    pub struct TunnelController {
        data_dir: PathBuf,
        active: AsyncMutex<Option<ActiveTunnel>>,
        route_names: RwLock<HashMap<String, String>>,
        last_status: RwLock<TunnelStatusDto>,
    }

    impl TunnelController {
        pub fn new(data_dir: PathBuf, initial_proxies: &[ProxyEntry]) -> Self {
            let route_names = compute_proxy_route_names(initial_proxies);
            Self {
                data_dir,
                active: AsyncMutex::new(None),
                route_names: RwLock::new(route_names),
                last_status: RwLock::new(TunnelStatusDto {
                    supported: true,
                    running: false,
                    state: "stopped".to_string(),
                    hostname: None,
                    public_urls: HashMap::new(),
                    connections: 0,
                    last_error: None,
                }),
            }
        }

        fn build_client(&self, api_url: Option<&str>) -> Result<Client, String> {
            let mut opts = ClientOptions::default();
            opts.storage = Some(Storage::at(self.data_dir.join("opentunnel")));
            if let Some(url_str) = api_url.map(str::trim).filter(|s| !s.is_empty()) {
                let parsed = url_str
                    .parse()
                    .map_err(|e| format!("Invalid OpenTunnel API URL '{url_str}': {e}"))?;
                opts.api = Some(parsed);
            }
            Ok(Client::new(opts))
        }

        fn build_opentunnel_routes(
            proxies: &[ProxyEntry],
            route_names: &HashMap<String, String>,
        ) -> Routes {
            let mut routes = Routes::new();
            for proxy in proxies {
                if !proxy.tunnel_enabled {
                    continue;
                }
                if let Some(name) = route_names.get(&proxy.id) {
                    let target = format!("{}:{}", proxy.target_host, proxy.target_port);
                    routes.insert(name.clone(), Route::new(target));
                }
            }
            routes
        }

        fn compute_public_urls(
            hostname: Option<&str>,
            route_names: &HashMap<String, String>,
        ) -> HashMap<String, String> {
            let Some(base_host) = hostname else {
                return HashMap::new();
            };
            route_names
                .iter()
                .map(|(proxy_id, route_name)| {
                    (
                        proxy_id.clone(),
                        format!("https://{route_name}.{base_host}"),
                    )
                })
                .collect()
        }

        fn format_state(state: State) -> String {
            match state {
                State::WaitingRoutes => "waiting-routes",
                State::Connecting => "connecting",
                State::Connected => "connected",
                State::Reconnecting => "reconnecting",
                State::Stopped => "stopped",
            }
            .to_string()
        }

        pub fn status(&self) -> TunnelStatusDto {
            self.last_status.read().clone()
        }

        pub async fn start(
            self: &Arc<Self>,
            proxies: &[ProxyEntry],
            api_url: Option<&str>,
            on_status: TunnelStatusCallback,
        ) -> Result<TunnelStatusDto, String> {
            // Stop any running tunnel first
            self.stop().await;

            let route_names = compute_proxy_route_names(proxies);
            *self.route_names.write() = route_names.clone();

            {
                let mut st = self.last_status.write();
                st.running = true;
                st.state = "provisioning".to_string();
                st.last_error = None;
                on_status(st.clone());
            }

            let client = self.build_client(api_url)?;
            let identity = client.ensure(PROFILE_NAME).await.map_err(|e| {
                let msg = format!("Failed to provision OpenTunnel identity: {e}");
                let mut st = self.last_status.write();
                st.running = false;
                st.state = "stopped".to_string();
                st.last_error = Some(msg.clone());
                on_status(st.clone());
                msg
            })?;

            let routes = Self::build_opentunnel_routes(proxies, &route_names);
            let tunnel = client.connect(PROFILE_NAME, routes).map_err(|e| {
                let msg = format!("Failed to connect OpenTunnel: {e}");
                let mut st = self.last_status.write();
                st.running = false;
                st.state = "stopped".to_string();
                st.last_error = Some(msg.clone());
                on_status(st.clone());
                msg
            })?;

            let hostname = identity.hostname.clone();
            let mut status_rx = tunnel.status_watch();

            let initial_ot = tunnel.status();
            let initial_dto = TunnelStatusDto {
                supported: true,
                running: true,
                state: Self::format_state(initial_ot.state),
                hostname: Some(hostname.clone()),
                public_urls: Self::compute_public_urls(Some(&hostname), &route_names),
                connections: initial_ot.connections,
                last_error: initial_ot.last_error,
            };
            *self.last_status.write() = initial_dto.clone();
            on_status(initial_dto.clone());

            *self.active.lock().await = Some(ActiveTunnel {
                tunnel,
                hostname: hostname.clone(),
            });

            // Watch live OpenTunnel state changes and push them to the frontend
            let controller = Arc::clone(self);
            tokio::spawn(async move {
                while status_rx.changed().await.is_ok() {
                    let ot = status_rx.borrow().clone();
                    let names = controller.route_names.read().clone();
                    let running = ot.state != State::Stopped;
                    let dto = TunnelStatusDto {
                        supported: true,
                        running,
                        state: Self::format_state(ot.state),
                        hostname: Some(ot.hostname.clone()),
                        public_urls: Self::compute_public_urls(Some(&ot.hostname), &names),
                        connections: ot.connections,
                        last_error: ot.last_error,
                    };
                    *controller.last_status.write() = dto.clone();
                    on_status(dto);
                    if !running {
                        break;
                    }
                }
            });

            Ok(initial_dto)
        }

        pub async fn stop(&self) -> TunnelStatusDto {
            let active = self.active.lock().await.take();
            if let Some(a) = active {
                let _ = a.tunnel.close().await;
            }
            let dto = TunnelStatusDto {
                supported: true,
                running: false,
                state: "stopped".to_string(),
                hostname: self.last_status.read().hostname.clone(),
                public_urls: HashMap::new(),
                connections: 0,
                last_error: None,
            };
            *self.last_status.write() = dto.clone();
            dto
        }

        pub async fn update_routes(
            &self,
            proxies: &[ProxyEntry],
        ) -> Result<TunnelStatusDto, String> {
            let route_names = compute_proxy_route_names(proxies);
            *self.route_names.write() = route_names.clone();

            let guard = self.active.lock().await;
            if let Some(active) = guard.as_ref() {
                let routes = Self::build_opentunnel_routes(proxies, &route_names);
                active
                    .tunnel
                    .set_routes(routes)
                    .map_err(|e| format!("Failed to update OpenTunnel routes: {e}"))?;
                let mut st = self.last_status.write();
                st.public_urls =
                    Self::compute_public_urls(Some(&active.hostname), &route_names);
                return Ok(st.clone());
            }

            Ok(self.last_status.read().clone())
        }
    }
}

#[cfg(not(feature = "tunnel"))]
mod imp {
    use super::*;

    pub struct TunnelController;

    impl TunnelController {
        pub fn new(_data_dir: PathBuf, _initial_proxies: &[ProxyEntry]) -> Self {
            Self
        }

        pub fn status(&self) -> TunnelStatusDto {
            TunnelStatusDto {
                supported: false,
                running: false,
                state: "unsupported".to_string(),
                hostname: None,
                public_urls: HashMap::new(),
                connections: 0,
                last_error: None,
            }
        }

        pub async fn start(
            self: &Arc<Self>,
            _proxies: &[ProxyEntry],
            _api_url: Option<&str>,
            _on_status: TunnelStatusCallback,
        ) -> Result<TunnelStatusDto, String> {
            Err(
                "Public tunnels are only available in LocalTLS Tunnel Edition. Download LocalTLS Tunnel to use OpenTunnel."
                    .to_string(),
            )
        }

        pub async fn stop(&self) -> TunnelStatusDto {
            self.status()
        }

        pub async fn update_routes(
            &self,
            _proxies: &[ProxyEntry],
        ) -> Result<TunnelStatusDto, String> {
            Ok(self.status())
        }
    }
}

pub use imp::TunnelController;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_proxy(id: &str, hostname: &str, tunnel_enabled: bool, route_name: Option<&str>) -> ProxyEntry {
        ProxyEntry {
            id: id.into(),
            hostname: hostname.into(),
            target_host: "127.0.0.1".into(),
            target_port: 3000,
            enabled: true,
            group_id: "default".into(),
            created_at: Utc::now(),
            tunnel_enabled,
            tunnel_route_name: route_name.map(String::from),
        }
    }

    #[test]
    fn test_route_name_derivation_and_deduplication() {
        let proxies = vec![
            sample_proxy("1", "api.myapp.test", true, None),
            sample_proxy("2", "api.other.test", true, None),
            sample_proxy("3", "web.myapp.test", false, None),
            sample_proxy("4", "foo.test", true, Some("My Custom_Route!")),
        ];

        let map = compute_proxy_route_names(&proxies);
        assert_eq!(map.get("1").map(String::as_str), Some("api"));
        assert_eq!(map.get("2").map(String::as_str), Some("api-2"));
        assert!(!map.contains_key("3"));
        assert_eq!(map.get("4").map(String::as_str), Some("my-custom-route"));
    }

    #[test]
    fn test_edition_info_matches_feature_flag() {
        let info = edition_info();
        #[cfg(feature = "tunnel")]
        {
            assert_eq!(info.edition, "tunnel");
            assert!(info.tunnel_supported);
        }
        #[cfg(not(feature = "tunnel"))]
        {
            assert_eq!(info.edition, "standard");
            assert!(!info.tunnel_supported);
        }
    }
}
