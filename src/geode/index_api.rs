use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;
use std::time::Duration;
use zip::ZipArchive;

use crate::core::worker::ModUpdateAvailable;
use crate::geode::package::InstalledMod;

const GEODE_API_BASE: &str = "https://api.geode-sdk.org/v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexModVersion {
    pub name: String,
    pub description: String,
    pub version: String,
    pub download_link: String,
    pub hash: String,
    pub geode: String,
    pub download_count: u64,
    pub gd_win: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexMod {
    pub id: String,
    pub name: String,
    pub description: String,
    pub developer: String,
    pub featured: bool,
    pub download_count: u64,
    pub tags: Vec<String>,
    pub source_url: Option<String>,
    pub versions: Vec<IndexModVersion>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoaderRelease {
    pub version: String,
    pub tag: String,
    pub gd_win: String,
    pub created_at: String,
    pub win_url: String,
    pub win_hash: String,
    pub resources_url: Option<String>,
    pub resources_hash: Option<String>,
}

fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(7))
        .timeout_read(Duration::from_secs(20))
        .build()
}

fn parse_gd_win(val: Option<&Value>) -> Option<String> {
    match val {
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Some(Value::Object(map)) => map
            .get("win")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string()),
        _ => None,
    }
}

fn parse_index_mod(v: &Value) -> Option<IndexMod> {
    let obj = v.as_object()?;
    let id = obj.get("id")?.as_str()?.to_string();
    let featured = obj.get("featured").and_then(|x| x.as_bool()).unwrap_or(false);
    let download_count = obj
        .get("download_count")
        .and_then(|x| x.as_u64())
        .unwrap_or(0);

    let developer = obj
        .get("developers")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|d| {
                    d.get("display_name")
                        .or_else(|| d.get("username"))
                        .and_then(|s| s.as_str())
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown".into());

    let tags = obj
        .get("tags")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let source_url = obj
        .get("links")
        .and_then(|l| l.get("source"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let mut versions = Vec::new();
    if let Some(ver_arr) = obj.get("versions").and_then(|x| x.as_array()) {
        for ver in ver_arr {
            let Some(vo) = ver.as_object() else { continue };
            let name = vo
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or(&id)
                .to_string();
            let description = vo
                .get("description")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let version = vo
                .get("version")
                .and_then(|x| x.as_str())
                .unwrap_or("0.0.0")
                .to_string();
            let download_link = vo
                .get("download_link")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let hash = vo
                .get("hash")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let geode = vo
                .get("geode")
                .and_then(|x| x.as_str())
                .unwrap_or("*")
                .to_string();
            let v_downloads = vo
                .get("download_count")
                .and_then(|x| x.as_u64())
                .unwrap_or(0);
            let status = vo
                .get("status")
                .and_then(|x| x.as_str())
                .unwrap_or("accepted")
                .to_string();
            let gd_win = parse_gd_win(vo.get("gd"));

            versions.push(IndexModVersion {
                name,
                description,
                version,
                download_link,
                hash,
                geode,
                download_count: v_downloads,
                gd_win,
                status,
            });
        }
    }

    let first = versions.first();
    Some(IndexMod {
        id: id.clone(),
        name: first.map(|f| f.name.clone()).unwrap_or(id),
        description: first.map(|f| f.description.clone()).unwrap_or_default(),
        developer,
        featured,
        download_count,
        tags,
        source_url,
        versions,
    })
}

pub fn search_index_mods(
    query: &str,
    gd_version: Option<&str>,
    sort: &str,
    featured_only: bool,
    page: usize,
    per_page: usize,
) -> Result<(Vec<IndexMod>, usize), String> {
    let agent = http_agent();
    let mut req = agent
        .get(&format!("{GEODE_API_BASE}/mods"))
        .query("platforms", "win")
        .query("page", &page.max(1).to_string())
        .query("per_page", &per_page.clamp(1, 50).to_string())
        .query("sort", if sort.is_empty() { "downloads" } else { sort });

    if !query.trim().is_empty() {
        req = req.query("query", query.trim());
    }
    if let Some(gd) = gd_version.filter(|s| !s.trim().is_empty()) {
        req = req.query("gd", gd.trim());
    }
    if featured_only {
        req = req.query("featured", "true");
    }

    let resp = req
        .call()
        .map_err(|e| format!("Geode Index search failed: {e}"))?;
    let json: Value = resp
        .into_json()
        .map_err(|e| format!("Invalid Geode Index JSON: {e}"))?;

    let payload = json
        .get("payload")
        .ok_or_else(|| "Missing payload in Geode Index response".to_string())?;
    let total = payload.get("count").and_then(|c| c.as_u64()).unwrap_or(0) as usize;
    let data = payload
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "Missing data array in Geode Index response".to_string())?;

    let mods = data.iter().filter_map(parse_index_mod).collect();
    Ok((mods, total))
}

pub fn fetch_mod_details(mod_id: &str) -> Result<IndexMod, String> {
    let agent = http_agent();
    let resp = agent
        .get(&format!("{GEODE_API_BASE}/mods/{mod_id}"))
        .call()
        .map_err(|e| format!("Fetch mod '{mod_id}' failed: {e}"))?;
    let json: Value = resp
        .into_json()
        .map_err(|e| format!("Invalid mod JSON: {e}"))?;
    let payload = json
        .get("payload")
        .ok_or_else(|| format!("Missing payload for {mod_id}"))?;
    parse_index_mod(payload).ok_or_else(|| format!("Could not parse mod entry for {mod_id}"))
}

pub fn fetch_latest_loader_release() -> Result<LoaderRelease, String> {
    let agent = http_agent();
    let resp = agent
        .get(&format!("{GEODE_API_BASE}/loader/versions/latest"))
        .query("platform", "win")
        .call()
        .map_err(|e| format!("Fetch latest Geode loader failed: {e}"))?;
    let json: Value = resp
        .into_json()
        .map_err(|e| format!("Parse Geode loader JSON: {e}"))?;
    let p = json
        .get("payload")
        .ok_or_else(|| "Missing payload in loader response".to_string())?;

    let downloads = p
        .get("downloads")
        .ok_or_else(|| "Missing downloads in loader response".to_string())?;
    let win = downloads
        .get("win")
        .ok_or_else(|| "Missing Windows loader download".to_string())?;
    let resources = downloads.get("resources");

    Ok(LoaderRelease {
        version: p
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        tag: p
            .get("tag")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        gd_win: parse_gd_win(p.get("gd")).unwrap_or_else(|| "latest".into()),
        created_at: p
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        win_url: win
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        win_hash: win
            .get("hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        resources_url: resources
            .and_then(|r| r.get("url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        resources_hash: resources
            .and_then(|r| r.get("hash"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

fn parse_semver(s: &str) -> Option<Version> {
    Version::parse(s.trim().trim_start_matches(['v', 'V'])).ok()
}

pub fn pick_compatible_version<'a>(
    index_mod: &'a IndexMod,
    gd_version: Option<&str>,
    geode_version: Option<&str>,
) -> Option<&'a IndexModVersion> {
    let loader_major = geode_version
        .and_then(parse_semver)
        .map(|v| v.major);

    index_mod.versions.iter().find(|v| {
        if v.status != "accepted" || v.download_link.is_empty() {
            return false;
        }
        // Known broken Windows release causing Entry Point Not Found crash on GD 2.2081
        if index_mod.id == "prevter.imageplus"
            && v.version.trim_start_matches(['v', 'V']) == "1.1.1"
            && gd_version == Some("2.2081")
        {
            return false;
        }
        let gd_ok = match (gd_version, v.gd_win.as_deref()) {
            (Some(active), Some(target)) if target != "*" && !target.is_empty() => {
                target == active
            }
            _ => true,
        };
        let geode_ok = match (loader_major, parse_semver(&v.geode)) {
            (Some(lm), Some(mv)) => lm == mv.major,
            _ => true,
        };
        gd_ok && geode_ok
    })
}

pub fn check_updates_for_installed(
    installed: &[InstalledMod],
    gd_version: Option<&str>,
    geode_version: Option<&str>,
) -> Vec<ModUpdateAvailable> {
    let mut updates = Vec::new();
    for m in installed {
        if m.version == "corrupt" {
            continue;
        }
        let Ok(remote) = fetch_mod_details(&m.id) else {
            continue;
        };
        let Some(best) = pick_compatible_version(&remote, gd_version, geode_version) else {
            continue;
        };
        if let (Some(cur_v), Some(rem_v)) = (parse_semver(&m.version), parse_semver(&best.version)) {
            if rem_v > cur_v {
                updates.push(ModUpdateAvailable {
                    mod_id: m.id.clone(),
                    mod_name: m.name.clone(),
                    current_version: m.version.clone(),
                    target_version: best.version.clone(),
                    download_link: best.download_link.clone(),
                    expected_hash: best.hash.clone(),
                    required_geode: best.geode.clone(),
                });
            }
        }
    }
    updates
}

pub fn download_verified_bytes(url: &str, expected_sha256: &str) -> Result<Vec<u8>, String> {
    let agent = http_agent();
    let resp = agent
        .get(url)
        .call()
        .map_err(|e| format!("Download failed ({url}): {e}"))?;
    let mut reader = resp.into_reader();
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Read stream failed: {e}"))?;

    if !expected_sha256.trim().is_empty() {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = format!("{:x}", hasher.finalize());
        if !actual.eq_ignore_ascii_case(expected_sha256.trim()) {
            return Err(format!(
                "SHA-256 verification failed! Expected {}, got {}",
                expected_sha256, actual
            ));
        }
    }

    Ok(bytes)
}

pub fn install_mod_from_index(
    game_dir: &str,
    mod_id: &str,
    gd_version: Option<&str>,
    geode_version: Option<&str>,
) -> Result<String, String> {
    let remote = fetch_mod_details(mod_id)?;
    let chosen = pick_compatible_version(&remote, gd_version, geode_version).ok_or_else(|| {
        format!(
            "No compatible release of {} found for GD {} / Geode {}.",
            remote.name,
            gd_version.unwrap_or("?"),
            geode_version.unwrap_or("?")
        )
    })?;

    install_mod_version(game_dir, mod_id, chosen)
}

pub fn install_mod_version(
    game_dir: &str,
    mod_id: &str,
    version: &IndexModVersion,
) -> Result<String, String> {
    let bytes = download_verified_bytes(&version.download_link, &version.hash)?;
    let geode_dir = crate::core::platform::gd_geode_dir(Path::new(game_dir));
    let mods_dir = geode_dir.join("mods");
    fs::create_dir_all(&mods_dir).map_err(|e| format!("create mods dir: {e}"))?;

    let tmp_path = mods_dir.join(format!("{mod_id}.geode.tmp"));
    let final_path = mods_dir.join(format!("{mod_id}.geode"));
    let disabled_path = mods_dir.join(format!("{mod_id}.geode.disabled"));

    fs::write(&tmp_path, &bytes).map_err(|e| format!("write tmp mod: {e}"))?;
    if disabled_path.exists() {
        let _ = fs::remove_file(&disabled_path);
    }
    fs::rename(&tmp_path, &final_path).map_err(|e| format!("finalize mod install: {e}"))?;
    let unzipped = geode_dir.join("unzipped").join(mod_id);
    if unzipped.is_dir() {
        let _ = fs::remove_dir_all(&unzipped);
    }
    Ok(format!("{} v{}", version.name, version.version))
}

pub fn install_specific_mod_version(
    game_dir: &str,
    mod_id: &str,
    version_tag: &str,
) -> Result<String, String> {
    let remote = fetch_mod_details(mod_id)?;
    let target = version_tag.trim_start_matches(['v', 'V']);
    let chosen = remote
        .versions
        .iter()
        .find(|v| v.version.trim_start_matches(['v', 'V']) == target)
        .ok_or_else(|| format!("Version {version_tag} not found for {mod_id}."))?;
    install_mod_version(game_dir, mod_id, chosen)
}

pub fn install_or_update_geode_loader(
    game_dir: &str,
    release: &LoaderRelease,
) -> Result<String, String> {
    let game_root = Path::new(game_dir);
    if !crate::core::platform::game_executable_exists(game_root) {
        return Err("Invalid Geometry Dash folder (executable not found).".into());
    }
    let bin_zip = download_verified_bytes(&release.win_url, &release.win_hash)?;
    extract_zip_into(&bin_zip, game_root)?;

    if let (Some(res_url), Some(res_hash)) = (&release.resources_url, &release.resources_hash) {
        let res_zip = download_verified_bytes(res_url, res_hash)?;
        let res_dir = game_root.join("geode").join("resources").join("geode.loader");
        fs::create_dir_all(&res_dir).map_err(|e| format!("create resources dir: {e}"))?;
        extract_zip_into(&res_zip, &res_dir)?;
    }

    Ok(format!(
        "Installed Geode {} (target GD {}) with SHA-256 verification.",
        release.version, release.gd_win
    ))
}

/// Fetches a mod's `logo.png` from Geode Index v1 API, caching the raw PNG in `%APPDATA%\GeodeDeck\cache\logos`.
pub fn fetch_mod_logo_rgba(mod_id: &str) -> Option<(u32, u32, Vec<u8>)> {
    let cache_dir = crate::core::config::config_dir()
        .join("cache")
        .join("logos");
    let safe_id: String = mod_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cache_path = cache_dir.join(format!("{safe_id}.png"));

    let png_bytes = if let Ok(cached) = fs::read(&cache_path) {
        cached
    } else {
        let url = format!("{GEODE_API_BASE}/mods/{mod_id}/logo");
        let resp = http_agent().get(&url).call().ok()?;
        let mut reader = resp.into_reader();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).ok()?;
        if !buf.is_empty() {
            let _ = fs::create_dir_all(&cache_dir);
            let _ = fs::write(&cache_path, &buf);
        }
        buf
    };

    let img = image::load_from_memory(&png_bytes).ok()?;
    let resized = img.thumbnail(128, 128).to_rgba8();
    let (w, h) = resized.dimensions();
    Some((w, h, resized.into_raw()))
}

fn extract_zip_into(zip_bytes: &[u8], dest_root: &Path) -> Result<(), String> {
    let mut archive =
        ZipArchive::new(Cursor::new(zip_bytes)).map_err(|e| format!("open zip: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("zip entry: {e}"))?;
        let Some(rel) = file.enclosed_name().map(|p| p.to_path_buf()) else {
            continue;
        };
        let out_path = dest_root.join(rel);
        if file.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| format!("mkdir: {e}"))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("mkdir parent: {e}"))?;
            }
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| format!("read zip entry: {e}"))?;
            fs::write(&out_path, &buf)
                .map_err(|e| format!("write {}: {e}", out_path.display()))?;
        }
    }
    Ok(())
}
