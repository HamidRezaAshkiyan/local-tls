use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub fn os_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macOS"
    }
    #[cfg(target_os = "windows")]
    {
        "Windows"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::env::consts::OS
    }
}

pub fn hosts_file_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        macos::hosts_file_path()
    }
    #[cfg(target_os = "windows")]
    {
        windows::hosts_file_path()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        PathBuf::from("/etc/hosts")
    }
}

pub fn write_hosts_file_elevated(data_dir: &Path, content: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::write_hosts_file_elevated(data_dir, content)
    }
    #[cfg(target_os = "windows")]
    {
        windows::write_hosts_file_elevated(data_dir, content)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (data_dir, content);
        Err("Unsupported operating system for automated hosts modification.".to_string())
    }
}

pub fn trust_ca_certificate(cert_path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::trust_ca_certificate(cert_path)
    }
    #[cfg(target_os = "windows")]
    {
        windows::trust_ca_certificate(cert_path)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = cert_path;
        Err("Unsupported operating system for automated certificate trust.".to_string())
    }
}

pub fn untrust_ca_certificate(common_name: &str, cert_path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::untrust_ca_certificate(common_name, cert_path)
    }
    #[cfg(target_os = "windows")]
    {
        windows::untrust_ca_certificate(common_name, cert_path)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (common_name, cert_path);
        Err("Unsupported operating system for automated certificate removal.".to_string())
    }
}

pub fn is_ca_certificate_trusted(common_name: &str, cert_path: &Path) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        macos::is_ca_certificate_trusted(common_name, cert_path)
    }
    #[cfg(target_os = "windows")]
    {
        windows::is_ca_certificate_trusted(common_name, cert_path)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (common_name, cert_path);
        Ok(false)
    }
}

pub fn open_in_file_manager(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::open_in_file_manager(path)
    }
    #[cfg(target_os = "windows")]
    {
        windows::open_in_file_manager(path)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = path;
        Err("Unsupported operating system.".to_string())
    }
}
