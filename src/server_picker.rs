use std::{process::Command, sync::Arc, thread};

use utils::{log, sync::Mutex};

#[derive(Debug, Clone)]
pub struct ServerRegion {
    pub name: String,
    pub description: String,
    pub relay_ips: Vec<String>,
    pub blocked: bool,
}

/// Passed through an Arc<Mutex<>> from the fetch thread to the UI thread.
/// `None` means the fetch is still running; `Some` carries the result.
pub type FetchResult = Arc<Mutex<Option<Result<Vec<ServerRegion>, String>>>>;

pub fn new_fetch_result() -> FetchResult {
    Arc::new(Mutex::new(None))
}

/// Kick off an async fetch and store the result in `out`.
pub fn fetch_servers_async(out: FetchResult) {
    thread::spawn(move || {
        let result = fetch_servers();
        *out.lock() = Some(result);
    });
}

fn fetch_servers() -> Result<Vec<ServerRegion>, String> {
    let output = Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "10",
            "https://api.steampowered.com/ISteamApps/GetSDRConfig/v1/?appid=1422450",
        ])
        .output()
        .map_err(|e| format!("Failed to execute curl: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "curl failed with exit code {}",
            output.status.code().unwrap_or(-1)
        ));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("JSON parse error: {e}"))?;

    let pops = json
        .get("pops")
        .and_then(|p| p.as_object())
        .ok_or_else(|| "Missing 'pops' field in API response".to_string())?;

    let mut regions: Vec<ServerRegion> = Vec::new();

    for (name, data) in pops {
        // skip entries without relay data
        let relays = match data.get("relays").and_then(|r| r.as_array()) {
            Some(r) => r,
            None => continue,
        };

        let description = data
            .get("desc")
            .and_then(|d| d.as_str())
            .unwrap_or(name.as_str())
            .to_string();

        let relay_ips: Vec<String> = relays
            .iter()
            .filter_map(|r| r.get("ipv4").and_then(|ip| ip.as_str()).map(String::from))
            .collect();

        if relay_ips.is_empty() {
            continue;
        }

        regions.push(ServerRegion {
            name: name.clone(),
            description,
            relay_ips,
            blocked: false,
        });
    }

    regions.sort_by(|a, b| a.description.cmp(&b.description));

    Ok(regions)
}

/// Block all relay IPs for a region using iptables.
pub fn block_region(relay_ips: &[String]) {
    for ip in relay_ips {
        run_iptables(&["-A", "INPUT", "-s", ip, "-j", "DROP"], ip, "block");
    }
}

/// Remove the iptables DROP rules for a region.
pub fn unblock_region(relay_ips: &[String]) {
    for ip in relay_ips {
        run_iptables(&["-D", "INPUT", "-s", ip, "-j", "DROP"], ip, "unblock");
    }
}

fn run_iptables(args: &[&str], ip: &str, action: &str) {
    let iptables = match find_binary(&[
        "/usr/sbin/iptables",
        "/sbin/iptables",
        "/usr/local/sbin/iptables",
        "iptables",
    ]) {
        Some(p) => p,
        None => {
            log::warn!("iptables binary not found; cannot {action} {ip}");
            return;
        }
    };

    // Try with sudo first; fall back to direct invocation (works when already root).
    let sudo = find_binary(&["/usr/bin/sudo", "/bin/sudo", "sudo"]);

    let status = if let Some(ref sudo_path) = sudo {
        Command::new(sudo_path).arg(&iptables).args(args).status()
    } else {
        Command::new(&iptables).args(args).status()
    };

    match status {
        Ok(s) if !s.success() => {
            log::warn!(
                "iptables {action} failed for {ip} (exit {})",
                s.code().unwrap_or(-1)
            );
        }
        Err(e) => {
            log::warn!("failed to run iptables for {ip}: {e}");
        }
        _ => {}
    }
}

/// Search for a binary by trying each candidate in order.
/// Candidates that are absolute paths are checked with [`std::path::Path::exists`];
/// bare names are resolved through the process `PATH` as a last resort.
fn find_binary(candidates: &[&str]) -> Option<std::path::PathBuf> {
    for &candidate in candidates {
        let path = std::path::Path::new(candidate);
        if path.is_absolute() {
            if path.exists() {
                return Some(path.to_path_buf());
            }
        } else {
            // Bare name — rely on PATH only as a fallback.
            if which_in_path(candidate) {
                return Some(std::path::PathBuf::from(candidate));
            }
        }
    }
    None
}

/// Returns `true` if `name` can be located via the current process `PATH`.
fn which_in_path(name: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            if dir.join(name).exists() {
                return true;
            }
        }
    }
    false
}
