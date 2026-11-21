use crate::platform;
use chrono::Local;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub const BLOCK_START: &str = "# >>> Ophiuchi-rs Managed Hosts >>>";
pub const BLOCK_END: &str = "# <<< Ophiuchi-rs Managed Hosts <<<";

/// Reads the current OS hosts file content.
pub fn read_hosts_file() -> Result<String, String> {
    let path = platform::hosts_file_path();
    fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read hosts file at {}: {e}", path.display()))
}

/// Renders a new hosts file string by replacing (or inserting) the LocalTLS managed block,
/// while preserving all unrelated user entries and line ending style (LF or CRLF).
pub fn render_hosts_with_managed_block(existing: &str, hostnames: &[String]) -> String {
    let line_ending = if existing.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };

    // Strip any existing managed block
    let mut kept_lines: Vec<&str> = Vec::new();
    let mut inside_block = false;
    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed == BLOCK_START {
            inside_block = true;
            continue;
        }
        if trimmed == BLOCK_END {
            inside_block = false;
            continue;
        }
        if !inside_block {
            kept_lines.push(line);
        }
    }

    // Trim trailing empty lines so we don't accumulate blank lines across syncs
    while kept_lines.last().is_some_and(|l| l.trim().is_empty()) {
        kept_lines.pop();
    }

    // Deduplicate and sort hostnames deterministically
    let unique_hosts: BTreeSet<String> = hostnames
        .iter()
        .map(|h| h.trim().to_ascii_lowercase())
        .filter(|h| !h.is_empty() && h != "localhost")
        .collect();

    let mut out = kept_lines.join(line_ending);
    if !out.is_empty() {
        out.push_str(line_ending);
    }

    if !unique_hosts.is_empty() {
        if !out.is_empty() {
            out.push_str(line_ending);
        }
        out.push_str(BLOCK_START);
        out.push_str(line_ending);
        for host in &unique_hosts {
            out.push_str(&format!("127.0.0.1 {host}{line_ending}"));
            out.push_str(&format!("::1 {host}{line_ending}"));
        }
        out.push_str(BLOCK_END);
        out.push_str(line_ending);
    }

    out
}

/// Returns which of the expected hostnames are missing a 127.0.0.1 entry in the hosts file.
pub fn find_unsynced_hosts(hosts_content: &str, expected_hostnames: &[String]) -> Vec<String> {
    let mut present_ipv4: BTreeSet<String> = BTreeSet::new();

    for raw_line in hosts_content.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        if let Some(ip) = parts.next() {
            if ip == "127.0.0.1" {
                for host in parts {
                    present_ipv4.insert(host.to_ascii_lowercase());
                }
            }
        }
    }

    let mut missing = Vec::new();
    for host in expected_hostnames {
        let normalized = host.trim().to_ascii_lowercase();
        if normalized.is_empty() || normalized == "localhost" {
            continue;
        }
        if !present_ipv4.contains(&normalized) {
            missing.push(normalized);
        }
    }
    missing.sort();
    missing.dedup();
    missing
}

/// Creates a timestamped backup of the current hosts file in `<data_dir>/backups/`.
pub fn backup_hosts_file(data_dir: &Path, existing_content: &str) -> Result<PathBuf, String> {
    let backups_dir = data_dir.join("backups");
    fs::create_dir_all(&backups_dir)
        .map_err(|e| format!("Failed to create backups directory: {e}"))?;
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let backup_path = backups_dir.join(format!("hosts.{stamp}.bak"));
    fs::write(&backup_path, existing_content)
        .map_err(|e| format!("Failed to write hosts backup: {e}"))?;
    Ok(backup_path)
}

/// Backs up and atomically updates the OS hosts file with the managed LocalTLS hostnames.
pub fn sync_hosts_file(data_dir: &Path, hostnames: &[String]) -> Result<(), String> {
    let existing = read_hosts_file()?;
    let updated = render_hosts_with_managed_block(&existing, hostnames);

    if existing == updated {
        return Ok(());
    }

    let _ = backup_hosts_file(data_dir, &existing)?;
    platform::write_hosts_file_elevated(data_dir, &updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_inserts_updates_and_removes_cleanly_lf() {
        let original = "127.0.0.1 localhost\n# User comment\n10.0.0.5 internal.corp\n";
        let rendered = render_hosts_with_managed_block(
            original,
            &["api.demo.test".into(), "web.demo.test".into()],
        );

        assert!(rendered.contains("127.0.0.1 localhost\n# User comment\n10.0.0.5 internal.corp\n"));
        assert!(rendered.contains("127.0.0.1 api.demo.test\n::1 api.demo.test\n"));
        assert!(rendered.contains("127.0.0.1 web.demo.test\n::1 web.demo.test\n"));

        // Idempotent second pass
        let rendered2 = render_hosts_with_managed_block(
            &rendered,
            &["web.demo.test".into(), "api.demo.test".into()],
        );
        assert_eq!(rendered, rendered2);

        // Update removes old host and keeps user entries
        let updated = render_hosts_with_managed_block(&rendered2, &["only.demo.test".into()]);
        assert!(!updated.contains("api.demo.test"));
        assert!(updated.contains("127.0.0.1 only.demo.test\n"));
        assert!(updated.contains("10.0.0.5 internal.corp"));

        // Clearing removes the entire managed block
        let cleared = render_hosts_with_managed_block(&updated, &[]);
        assert!(!cleared.contains(BLOCK_START));
        assert!(!cleared.contains(BLOCK_END));
        assert!(cleared.contains("10.0.0.5 internal.corp"));
    }

    #[test]
    fn test_render_preserves_windows_crlf() {
        let original = "127.0.0.1 localhost\r\n# Windows hosts\r\n";
        let rendered = render_hosts_with_managed_block(original, &["app.local.test".into()]);
        assert!(rendered.contains("\r\n"));
        assert!(rendered.contains("# >>> Ophiuchi-rs Managed Hosts >>>\r\n127.0.0.1 app.local.test\r\n"));
    }

    #[test]
    fn test_find_unsynced_hosts() {
        let hosts = "127.0.0.1 localhost\n# 127.0.0.1 commented.test\n127.0.0.1 synced.test\n";
        let missing = find_unsynced_hosts(
            hosts,
            &[
                "synced.test".into(),
                "commented.test".into(),
                "missing.test".into(),
                "localhost".into(),
            ],
        );
        assert_eq!(missing, vec!["commented.test", "missing.test"]);
    }
}
