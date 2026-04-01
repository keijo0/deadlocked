use std::{net::Ipv4Addr, process::Command, sync::Arc, thread};

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

/// Maps a Steam SDR datacenter pop code to its geographic continent.
///
/// CS2 uses standard 3-letter IATA/city codes (e.g. `"lhr"`, `"iad"`, `"sto"`).
/// Some pop codes may include numeric suffixes or non-IATA aliases, so this
/// function normalises the code by stripping any trailing ASCII digits before
/// matching, so `"maa2"` is treated identically to `"maa"`.
///
/// Codes that are not recognised (after normalisation) return
/// [`Continent::Unknown`].
fn continent_from_name(name: &str) -> Continent {
    // Strip trailing digits so numbered variants (e.g. "maa2", "bom2") match
    // their base code.
    let lower = name.to_lowercase();
    let normalised = lower.trim_end_matches(|c: char| c.is_ascii_digit());

    match normalised {
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
        // Asia — includes non-IATA city codes used by Valve's SDR:
        //   seo = Seoul (Valve uses "seo", IATA is "icn")
        //   tyo = Tokyo (common Valve alias; IATA is "nrt"/"hnd")
        "sgp" | "hkg" | "tyo" | "nrt" | "osk" | "bom" | "del" | "maa" | "ccu" | "hyb"
        | "bkk" | "kul" | "icn" | "seo" | "sha" | "pek" | "can" | "szx" | "pnq" | "blr"
        | "amd" => Continent::Asia,
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

/// Fallback continent classifier that inspects the human-readable description
/// returned by the Steam SDR API (e.g. `"Stockholm - Kista"`).  Used when the
/// pop code is not recognised by [`continent_from_name`] so that newly added or
/// non-standard pops are still placed in the right continent group rather than
/// being silently dropped into `Unknown` (which users can easily miss when
/// clicking continent-level "Block All" buttons).
fn continent_from_description(desc: &str) -> Continent {
    let d = desc.to_lowercase();

    // North America — cities / regions
    if d.contains("ashburn")
        || d.contains("chicago")
        || d.contains("los angeles")
        || d.contains("seattle")
        || d.contains("atlanta")
        || d.contains("dallas")
        || d.contains("miami")
        || d.contains("denver")
        || d.contains("portland")
        || d.contains("san jose")
        || d.contains("oklahoma")
        || d.contains("toronto")
        || d.contains("calgary")
        || d.contains("montreal")
        || d.contains("vancouver")
        || d.contains("mexico")
        || d.contains("fayetteville")
        || d.contains("north america")
    {
        return Continent::NorthAmerica;
    }

    // South America
    if d.contains("sao paulo")
        || d.contains("são paulo")
        || d.contains("rio")
        || d.contains("santiago")
        || d.contains("lima")
        || d.contains("bogota")
        || d.contains("bogotá")
        || d.contains("buenos aires")
        || d.contains("south america")
    {
        return Continent::SouthAmerica;
    }

    // Europe — cities / countries (covers Valve's sub-city pop names like
    // "Stockholm - Kista", "Stockholm - Bromma", etc.)
    if d.contains("stockholm")
        || d.contains("sweden")
        || d.contains("london")
        || d.contains("amsterdam")
        || d.contains("frankfurt")
        || d.contains("paris")
        || d.contains("madrid")
        || d.contains("vienna")
        || d.contains("warsaw")
        || d.contains("prague")
        || d.contains("helsinki")
        || d.contains("budapest")
        || d.contains("zurich")
        || d.contains("milan")
        || d.contains("lisbon")
        || d.contains("athens")
        || d.contains("oslo")
        || d.contains("copenhagen")
        || d.contains("dublin")
        || d.contains("brussels")
        || d.contains("munich")
        || d.contains("berlin")
        || d.contains("hamburg")
        || d.contains("dusseldorf")
        || d.contains("düsseldorf")
        || d.contains("tallinn")
        || d.contains("riga")
        || d.contains("vilnius")
        || d.contains("manchester")
        || d.contains("europe")
    {
        return Continent::Europe;
    }

    // Asia
    if d.contains("singapore")
        || d.contains("hong kong")
        || d.contains("tokyo")
        || d.contains("osaka")
        || d.contains("mumbai")
        || d.contains("delhi")
        || d.contains("chennai")
        || d.contains("kolkata")
        || d.contains("hyderabad")
        || d.contains("bangkok")
        || d.contains("kuala lumpur")
        || d.contains("seoul")
        || d.contains("shanghai")
        || d.contains("beijing")
        || d.contains("guangzhou")
        || d.contains("shenzhen")
        || d.contains("pune")
        || d.contains("bangalore")
        || d.contains("bengaluru")
        || d.contains("ahmedabad")
        || d.contains("asia")
    {
        return Continent::Asia;
    }

    // Middle East
    if d.contains("dubai")
        || d.contains("bahrain")
        || d.contains("karachi")
        || d.contains("kuwait")
        || d.contains("tel aviv")
        || d.contains("istanbul")
        || d.contains("ankara")
        || d.contains("riyadh")
        || d.contains("abu dhabi")
        || d.contains("middle east")
    {
        return Continent::MiddleEast;
    }

    // Africa
    if d.contains("johannesburg")
        || d.contains("lagos")
        || d.contains("nairobi")
        || d.contains("cairo")
        || d.contains("accra")
        || d.contains("dakar")
        || d.contains("africa")
    {
        return Continent::Africa;
    }

    // Oceania
    if d.contains("sydney")
        || d.contains("melbourne")
        || d.contains("perth")
        || d.contains("brisbane")
        || d.contains("adelaide")
        || d.contains("auckland")
        || d.contains("canberra")
        || d.contains("australia")
        || d.contains("new zealand")
        || d.contains("oceania")
    {
        return Continent::Oceania;
    }

    Continent::Unknown
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
            "https://api.steampowered.com/ISteamApps/GetSDRConfig/v1/?appid=730",
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
            .flat_map(|r| {
                let ip_str = match r.get("ipv4").and_then(|ip| ip.as_str()) {
                    Some(s) => s,
                    None => return vec![],
                };
                // `num_addresses` is the count of consecutive relay IPs starting at
                // `ipv4`.  When absent or explicitly 0, treat it as 1 so the base
                // IP is always included.  Cap at u32::MAX to avoid truncation when
                // converting from u64 (Steam returns small values in practice, but
                // be safe).
                let count_u64 = r
                    .get("num_addresses")
                    .and_then(|n| n.as_u64())
                    .unwrap_or(1)
                    .max(1);
                let count = if count_u64 > u32::MAX as u64 {
                    log::warn!(
                        "relay {ip_str:?} has num_addresses={count_u64} which exceeds u32::MAX; capping"
                    );
                    u32::MAX
                } else {
                    count_u64 as u32
                };
                let base: Ipv4Addr = match ip_str.parse() {
                    Ok(ip) => ip,
                    Err(e) => {
                        log::warn!("skipping unparseable relay IP {ip_str:?}: {e}");
                        return vec![];
                    }
                };
                let base_u32 = u32::from(base);
                (0..count)
                    .filter_map(|i| {
                        base_u32.checked_add(i).map(|n| Ipv4Addr::from(n).to_string())
                    })
                    .collect()
            })
            .collect();

        if relay_ips.is_empty() {
            continue;
        }

        regions.push(ServerRegion {
            continent: {
                let by_code = continent_from_name(name);
                if by_code == Continent::Unknown {
                    continent_from_description(&description)
                } else {
                    by_code
                }
            },
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
/// Both directions are dropped so the game client cannot reach the relay
/// (OUTPUT) and cannot receive traffic from it (INPUT).
///
/// Rules are inserted at position 1 (`-I … 1`) rather than appended (`-A`)
/// so that they take priority over any pre-existing ACCEPT rules in the
/// chain (e.g. conntrack ESTABLISHED/RELATED rules added by ufw or
/// firewalld).  Without this, appended DROP rules would never be evaluated
/// for packets that already match an earlier ACCEPT rule.
pub fn block_region(relay_ips: &[String]) {
    for ip in relay_ips {
        run_iptables(
            &["-I", "INPUT", "1", "-s", ip, "-j", "DROP"],
            ip,
            "block INPUT",
        );
        run_iptables(
            &["-I", "OUTPUT", "1", "-d", ip, "-j", "DROP"],
            ip,
            "block OUTPUT",
        );
    }
}

/// Remove the iptables DROP rules for a region.
pub fn unblock_region(relay_ips: &[String]) {
    for ip in relay_ips {
        run_iptables(&["-D", "INPUT", "-s", ip, "-j", "DROP"], ip, "unblock INPUT");
        run_iptables(&["-D", "OUTPUT", "-d", ip, "-j", "DROP"], ip, "unblock OUTPUT");
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
        Ok(s) if s.success() => {
            log::debug!("iptables {action} succeeded for {ip}");
        }
        Ok(s) => {
            log::warn!(
                "iptables {action} failed for {ip} (exit {})",
                s.code().unwrap_or(-1)
            );
        }
        Err(e) => {
            log::warn!("failed to run iptables for {ip}: {e}");
        }
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
