use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn hosts_file_path() -> PathBuf {
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    PathBuf::from(system_root)
        .join("System32")
        .join("drivers")
        .join("etc")
        .join("hosts")
}

fn ps_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

pub fn write_hosts_file_elevated(data_dir: &Path, content: &str) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("Failed to create data dir: {e}"))?;
    let staged = data_dir.join("hosts.staged");
    fs::write(&staged, content).map_err(|e| format!("Failed to write staged hosts file: {e}"))?;

    let target = hosts_file_path();
    let inner_cmd = format!(
        "Copy-Item -LiteralPath {} -Destination {} -Force",
        ps_single_quote(&staged.to_string_lossy()),
        ps_single_quote(&target.to_string_lossy())
    );

    let outer_script = format!(
        "$p = Start-Process powershell -Verb RunAs -Wait -PassThru -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-Command',{}; exit $p.ExitCode",
        ps_single_quote(&inner_cmd)
    );

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &outer_script,
        ])
        .output()
        .map_err(|e| format!("Failed to launch elevated PowerShell: {e}"))?;

    let _ = fs::remove_file(&staged);

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!(
            "Failed to update Windows hosts file (UAC may have been cancelled): {stderr}"
        ))
    }
}

/// Installs the LocalTLS Root CA into the Current User Trusted Root Certification Authorities store.
/// This requires no UAC elevation and is trusted by Chrome, Edge, Brave, and Windows TLS APIs.
pub fn trust_ca_certificate(cert_path: &Path) -> Result<(), String> {
    let output = Command::new("certutil")
        .args(["-user", "-addstore", "Root"])
        .arg(cert_path)
        .output()
        .map_err(|e| format!("Failed to run certutil: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(format!(
            "certutil failed to trust certificate: {stderr} {stdout}"
        ))
    }
}

pub fn untrust_ca_certificate(common_name: &str, _cert_path: &Path) -> Result<(), String> {
    let output = Command::new("certutil")
        .args(["-user", "-delstore", "Root", common_name])
        .output()
        .map_err(|e| format!("Failed to run certutil: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(format!(
            "certutil failed to remove certificate: {stderr} {stdout}"
        ))
    }
}

pub fn is_ca_certificate_trusted(common_name: &str, cert_path: &Path) -> Result<bool, String> {
    if !cert_path.exists() {
        return Ok(false);
    }

    let output = Command::new("certutil")
        .args(["-user", "-verifystore", "Root", common_name])
        .output()
        .map_err(|e| format!("Failed to query Windows certificate store: {e}"))?;

    Ok(output.status.success())
}

pub fn open_in_file_manager(path: &Path) -> Result<(), String> {
    let _ = Command::new("explorer")
        .arg(path)
        .spawn()
        .map_err(|e| format!("Failed to open Explorer: {e}"))?;
    Ok(())
}
