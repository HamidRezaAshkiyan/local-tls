use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn hosts_file_path() -> PathBuf {
    PathBuf::from("/etc/hosts")
}

fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn run_privileged_shell(shell_cmd: &str) -> Result<(), String> {
    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        escape_applescript_string(shell_cmd)
    );
    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("Failed to launch macOS authorization prompt: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.contains("User canceled") || stderr.contains("-128") {
            Err("Administrator authorization was cancelled by the user.".to_string())
        } else {
            Err(format!("Privileged macOS command failed: {stderr}"))
        }
    }
}

pub fn write_hosts_file_elevated(data_dir: &Path, content: &str) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("Failed to create data dir: {e}"))?;
    let staged = data_dir.join("hosts.staged");
    fs::write(&staged, content).map_err(|e| format!("Failed to write staged hosts file: {e}"))?;

    let staged_str = staged.to_string_lossy();
    let cmd = format!(
        "cp {} /etc/hosts && chmod 644 /etc/hosts",
        shell_single_quote(&staged_str)
    );
    let res = run_privileged_shell(&cmd);
    let _ = fs::remove_file(&staged);
    res
}

pub fn trust_ca_certificate(cert_path: &Path) -> Result<(), String> {
    let cert_str = cert_path.to_string_lossy();
    let cmd = format!(
        "security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain {}",
        shell_single_quote(&cert_str)
    );
    run_privileged_shell(&cmd)
}

pub fn untrust_ca_certificate(common_name: &str, cert_path: &Path) -> Result<(), String> {
    let cert_str = cert_path.to_string_lossy();
    let cmd = format!(
        "security remove-trusted-cert -d {} 2>/dev/null || true; security delete-certificate -c {} /Library/Keychains/System.keychain 2>/dev/null || true",
        shell_single_quote(&cert_str),
        shell_single_quote(common_name)
    );
    run_privileged_shell(&cmd)
}

pub fn is_ca_certificate_trusted(common_name: &str, cert_path: &Path) -> Result<bool, String> {
    if !cert_path.exists() {
        return Ok(false);
    }

    // First verify the certificate is present in System.keychain
    let find_out = Command::new("security")
        .args([
            "find-certificate",
            "-c",
            common_name,
            "/Library/Keychains/System.keychain",
        ])
        .output()
        .map_err(|e| format!("Failed to query macOS Keychain: {e}"))?;

    if !find_out.status.success() {
        return Ok(false);
    }

    // Verify trust evaluation passes
    let verify_out = Command::new("security")
        .args(["verify-cert", "-c"])
        .arg(cert_path)
        .output()
        .map_err(|e| format!("Failed to verify certificate trust: {e}"))?;

    Ok(verify_out.status.success())
}

pub fn open_in_file_manager(path: &Path) -> Result<(), String> {
    let status = Command::new("open")
        .arg(path)
        .status()
        .map_err(|e| format!("Failed to open Finder: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Failed to open path in Finder: {}", path.display()))
    }
}
