use parking_lot::RwLock;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose, SanType,
};
use rustls::crypto::ring::sign::any_supported_type;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};

pub const CA_COMMON_NAME: &str = "Ophiuchi-rs Local Root CA";
pub const CA_ORGANIZATION: &str = "Ophiuchi-rs";

struct RootCa {
    cert: Certificate,
    key: KeyPair,
    cert_der: CertificateDer<'static>,
}

pub struct CertManager {
    data_dir: PathBuf,
    ca: RwLock<Option<RootCa>>,
    leaf_cache: RwLock<HashMap<String, Arc<CertifiedKey>>>,
}

impl std::fmt::Debug for CertManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CertManager")
            .field("data_dir", &self.data_dir)
            .finish()
    }
}

impl CertManager {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            ca: RwLock::new(None),
            leaf_cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn ca_dir(&self) -> PathBuf {
        self.data_dir.join("certs").join("ca")
    }

    pub fn ca_cert_path(&self) -> PathBuf {
        self.ca_dir().join("rootCA.pem")
    }

    pub fn ca_key_path(&self) -> PathBuf {
        self.ca_dir().join("rootCA.key")
    }

    pub fn host_cert_dir(&self, hostname: &str) -> PathBuf {
        self.data_dir.join("certs").join("hosts").join(hostname)
    }

    pub fn ca_exists(&self) -> bool {
        self.ca_cert_path().exists() && self.ca_key_path().exists()
    }

    /// Loads the existing Local Root CA from disk or generates a new ECDSA P-256 Root CA.
    pub fn ensure_root_ca(&self) -> Result<PathBuf, String> {
        {
            let guard = self.ca.read();
            if guard.is_some() && self.ca_exists() {
                return Ok(self.ca_cert_path());
            }
        }

        let ca_dir = self.ca_dir();
        fs::create_dir_all(&ca_dir)
            .map_err(|e| format!("Failed to create CA directory: {e}"))?;

        let cert_path = self.ca_cert_path();
        let key_path = self.ca_key_path();

        let root_ca = if cert_path.exists() && key_path.exists() {
            let cert_pem = fs::read_to_string(&cert_path)
                .map_err(|e| format!("Failed to read Root CA cert: {e}"))?;
            let key_pem = fs::read_to_string(&key_path)
                .map_err(|e| format!("Failed to read Root CA key: {e}"))?;

            let key = KeyPair::from_pem(&key_pem)
                .map_err(|e| format!("Failed to parse Root CA private key: {e}"))?;
            let params = CertificateParams::from_ca_cert_pem(&cert_pem)
                .map_err(|e| format!("Failed to parse Root CA certificate: {e}"))?;
            let cert = params
                .self_signed(&key)
                .map_err(|e| format!("Failed to reconstruct Root CA: {e}"))?;

            let mut reader = std::io::BufReader::new(cert_pem.as_bytes());
            let der_certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to decode Root CA DER: {e}"))?;
            let cert_der = der_certs
                .into_iter()
                .next()
                .ok_or_else(|| "Root CA PEM contained no certificates".to_string())?;

            RootCa {
                cert,
                key,
                cert_der,
            }
        } else {
            let mut params = CertificateParams::default();
            params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
            params.distinguished_name.push(DnType::CommonName, CA_COMMON_NAME);
            params
                .distinguished_name
                .push(DnType::OrganizationName, CA_ORGANIZATION);
            params.key_usages = vec![
                KeyUsagePurpose::KeyCertSign,
                KeyUsagePurpose::CrlSign,
                KeyUsagePurpose::DigitalSignature,
            ];
            let now = OffsetDateTime::now_utc();
            params.not_before = now - Duration::days(1);
            params.not_after = now + Duration::days(3650);

            let key = KeyPair::generate()
                .map_err(|e| format!("Failed to generate Root CA keypair: {e}"))?;
            let cert = params
                .self_signed(&key)
                .map_err(|e| format!("Failed to self-sign Root CA: {e}"))?;

            fs::write(&cert_path, cert.pem())
                .map_err(|e| format!("Failed to save Root CA cert: {e}"))?;
            fs::write(&key_path, key.serialize_pem())
                .map_err(|e| format!("Failed to save Root CA key: {e}"))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600));
            }

            let cert_der = cert.der().clone();
            RootCa {
                cert,
                key,
                cert_der,
            }
        };

        let mut guard = self.ca.write();
        *guard = Some(root_ca);
        Ok(cert_path)
    }

    pub fn ca_cert_pem(&self) -> Result<String, String> {
        let path = self.ensure_root_ca()?;
        fs::read_to_string(&path).map_err(|e| format!("Failed to read Root CA PEM: {e}"))
    }

    /// Generates (or loads from memory cache) a leaf TLS certificate signed by our Local Root CA
    /// for the requested hostname.
    pub fn get_or_create_leaf(&self, hostname: &str) -> Result<Arc<CertifiedKey>, String> {
        let normalized = hostname.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Err("Empty hostname for leaf certificate".to_string());
        }

        if let Some(existing) = self.leaf_cache.read().get(&normalized) {
            return Ok(Arc::clone(existing));
        }

        self.ensure_root_ca()?;

        let (leaf_cert_pem, leaf_key_pem, cert_chain, key_der) = {
            let ca_guard = self.ca.read();
            let ca = ca_guard
                .as_ref()
                .ok_or_else(|| "Root CA not initialized".to_string())?;

            let mut params = CertificateParams::default();
            params.is_ca = IsCa::NoCa;
            params
                .distinguished_name
                .push(DnType::CommonName, normalized.clone());
            params
                .distinguished_name
                .push(DnType::OrganizationName, CA_ORGANIZATION);
            params.key_usages = vec![
                KeyUsagePurpose::DigitalSignature,
                KeyUsagePurpose::KeyEncipherment,
            ];
            params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

            let dns_name = normalized
                .clone()
                .try_into()
                .map_err(|e| format!("Invalid DNS SAN '{normalized}': {e}"))?;
            params.subject_alt_names = vec![
                SanType::DnsName(dns_name),
                SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            ];

            // Apple & Chrome enforce max 825-day validity on leaf certs
            let now = OffsetDateTime::now_utc();
            params.not_before = now - Duration::hours(1);
            params.not_after = now + Duration::days(820);

            let leaf_key = KeyPair::generate()
                .map_err(|e| format!("Failed to generate leaf keypair: {e}"))?;
            let leaf_cert = params
                .signed_by(&leaf_key, &ca.cert, &ca.key)
                .map_err(|e| format!("Failed to sign leaf cert for '{normalized}': {e}"))?;

            let cert_chain = vec![leaf_cert.der().clone(), ca.cert_der.clone()];
            let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der()));
            (leaf_cert.pem(), leaf_key.serialize_pem(), cert_chain, key_der)
        };

        // Persist leaf cert to disk for inspection/debugging
        let host_dir = self.host_cert_dir(&normalized);
        if fs::create_dir_all(&host_dir).is_ok() {
            let _ = fs::write(host_dir.join("cert.pem"), &leaf_cert_pem);
            let _ = fs::write(host_dir.join("key.pem"), &leaf_key_pem);
        }

        let signing_key = any_supported_type(&key_der)
            .map_err(|e| format!("Failed to load leaf signing key: {e}"))?;
        let certified = Arc::new(CertifiedKey::new(cert_chain, signing_key));

        self.leaf_cache
            .write()
            .insert(normalized, Arc::clone(&certified));

        Ok(certified)
    }

    pub fn remove_host_cert(&self, hostname: &str) {
        let normalized = hostname.trim().to_ascii_lowercase();
        self.leaf_cache.write().remove(&normalized);
        let dir = self.host_cert_dir(&normalized);
        if dir.exists() {
            let _ = fs::remove_dir_all(dir);
        }
    }

    pub fn clear_leaf_cache(&self) {
        self.leaf_cache.write().clear();
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

/// SNI resolver for `tokio-rustls` that dynamically serves a LocalTLS-signed leaf certificate
/// matching the TLS `ClientHello` server name.
#[derive(Debug)]
pub struct DynamicSniResolver {
    cert_manager: Arc<CertManager>,
}

impl DynamicSniResolver {
    pub fn new(cert_manager: Arc<CertManager>) -> Self {
        Self { cert_manager }
    }
}

impl ResolvesServerCert for DynamicSniResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let hostname = client_hello.server_name().unwrap_or("localhost");
        self.cert_manager.get_or_create_leaf(hostname).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_root_ca_and_leaf_cert_generation() {
        let dir = TempDir::new().unwrap();
        let mgr = CertManager::new(dir.path().to_path_buf());

        let ca_path = mgr.ensure_root_ca().unwrap();
        assert!(ca_path.exists());
        assert!(mgr.ca_key_path().exists());

        let leaf = mgr.get_or_create_leaf("api.localtls.test").unwrap();
        assert_eq!(leaf.cert.len(), 2); // leaf + root CA chain

        // Reloading CertManager from the same directory reuses the persisted Root CA
        let mgr2 = CertManager::new(dir.path().to_path_buf());
        assert!(mgr2.ca_exists());
        let leaf2 = mgr2.get_or_create_leaf("web.localtls.test").unwrap();
        assert_eq!(leaf2.cert.len(), 2);
        assert!(dir
            .path()
            .join("certs/hosts/web.localtls.test/cert.pem")
            .exists());
    }
}
