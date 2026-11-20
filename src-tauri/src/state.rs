use crate::certs::CertManager;
use crate::models::{AppConfig, ProxyLogEntry};
use crate::proxy::ProxyController;
use crate::storage;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

pub const MAX_LOG_ENTRIES: usize = 500;

pub struct AppState {
    pub data_dir: PathBuf,
    pub config: RwLock<AppConfig>,
    pub cert_manager: Arc<CertManager>,
    pub proxy: Arc<ProxyController>,
    pub logs: RwLock<VecDeque<ProxyLogEntry>>,
}

impl AppState {
    pub fn initialize(data_dir: PathBuf) -> Result<Self, String> {
        let config = storage::load_config(&data_dir)?;
        let cert_manager = Arc::new(CertManager::new(data_dir.clone()));
        let proxy = Arc::new(ProxyController::new(&config.proxies));

        // Pre-generate Local Root CA on first boot (no privilege prompt yet; user chooses when to trust)
        let _ = cert_manager.ensure_root_ca();

        Ok(Self {
            data_dir,
            config: RwLock::new(config),
            cert_manager,
            proxy,
            logs: RwLock::new(VecDeque::with_capacity(MAX_LOG_ENTRIES)),
        })
    }

    pub fn push_log(&self, entry: ProxyLogEntry) {
        let mut logs = self.logs.write();
        if logs.len() >= MAX_LOG_ENTRIES {
            logs.pop_front();
        }
        logs.push_back(entry);
    }

    pub fn save_current_config(&self) -> Result<(), String> {
        let cfg = self.config.read().clone();
        storage::save_config(&self.data_dir, &cfg)
    }

    pub fn active_hostnames(&self) -> Vec<String> {
        self.config
            .read()
            .proxies
            .iter()
            .filter(|p| p.enabled)
            .map(|p| p.hostname.clone())
            .collect()
    }
}
