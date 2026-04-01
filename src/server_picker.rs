use std::{process::Command, sync::Arc, thread};

use utils::{log, sync::Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Continent {
    Africa,
    Asia,
    Europe,
    MiddleEast,
    NorthAmerica,
    Oceania,
    SouthAmerica,
    Unknown,
}

impl Continent {
    pub fn as_str(self) -> &'static str {
        match self {
            Continent::Africa => "Africa",
            Continent::Asia => "Asia",
            Continent::Europe => "Europe",
            Continent::MiddleEast => "Middle East",
            Continent::NorthAmerica => "North America",
            Continent::Oceania => "Oceania",
            Continent::SouthAmerica => "South America",
            Continent::Unknown => "Unknown",
        }
    }
}

/// Maps a Steam relay datacenter code (IATA airport/city code) to its continent.
/// The input `name` is expected to be a lowercase 3-letter airport/city code as
/// reported by the Steam server picker (e.g. `"iad"`, `"lhr"`, `"sgp"`).
/// Returns `Continent::Unknown` for any code not in the mapping table.
fn continent_from_name(name: &str) -> Continent {
    match name.to_lowercase().as_str() {
        // North America
        "iad" | "ord" | "lax" | "sea" | "atl" | "dfw" | "mia" | "den" | "pdx" | "sjc"
        | "okc" | "ytz" | "yyc" | "yul" | "yvr" | "mex" | "xna" => Continent::NorthAmerica,
        // South America
        "gru" | "gig" | "scl" | "lim" | "bog" | "bue" | "eze" => Continent::SouthAmerica,
        // Europe
        "lhr" | "ams" | "fra" | "par" | "mad" | "sto" | "vie" | "waw" | "prg" | "hel"
        | "bud" | "zur" | "zrh" | "mil" | "lis" | "ath" | "osl" | "cph" | "dub" | "arn"
        | "man" | "bru" | "muc" | "cdg" | "ber" | "ham" | "dus" | "tll" | "rig" | "vno" => {
            Continent::Europe
        }
        // Asia
        "sgp" | "hkg" | "tyo" | "nrt" | "osk" | "bom" | "del" | "maa" | "ccu" | "hyb"
        | "bkk" | "kul" | "icn" | "sha" | "pek" | "can" | "szx" | "pnq" | "blr" | "amd" => {
            Continent::Asia
        }
        // Middle East
        "dxb" | "bah" | "khi" | "kwi" | "tlv" | "ist" | "esb" | "ruh" | "auh" => {
            Continent::MiddleEast
        }
        // Africa
        "jnb" | "lag" | "nbo" | "cai" | "acc" | "dkr" => Continent::Africa,
        // Oceania
        "syd" | "mel" | "per" | "bne" | "adl" | "akl" | "cbr" => Continent::Oceania,
        _ => Continent::Unknown,
    }
}

#[derive(Debug, Clone)]
pub struct ServerRegion {
    pub name: String,
    pub description: String,
    pub relay_ips: Vec<String>,
    pub blocked: bool,
    pub continent: Continent,
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
            continent: continent_from_name(name),
            name: name.clone(),
            description,
            relay_ips,
            blocked: false,
        });
    }

    regions.sort_by(|a, b| {
        a.continent
            .cmp(&b.continent)
            .then_with(|| a.description.cmp(&b.description))
    });

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
    let status = Command::new("sudo").arg("iptables").args(args).status();

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
