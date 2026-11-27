use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

pub const APP_RELEASE_TAG: &str = "Alpha-1.0.0";

#[derive(Clone, Debug)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_url: String,
    pub release_notes: String,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub expected_sha256: Option<String>,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    name: String,
    html_url: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub digest: Option<String>,
}

pub fn select_platform_asset_for<'a>(
    assets: &'a [GithubAsset],
    target_os: &str,
    target_arch: &str,
) -> Option<&'a GithubAsset> {
    let candidates: Vec<&GithubAsset> = assets
        .iter()
        .filter(|a| {
            let l = a.name.to_lowercase();
            !l.ends_with(".sha256") && l != "sha256sums.txt"
        })
        .collect();

    match target_os {
        "windows" => candidates
            .iter()
            .copied()
            .find(|a| {
                let l = a.name.to_lowercase();
                (l.contains("windows") || l.contains("win")) && l.ends_with(".exe")
            })
            .or_else(|| {
                candidates.iter().copied().find(|a| {
                    let l = a.name.to_lowercase();
                    (l.contains("windows") || l.contains("win")) && l.ends_with(".zip")
                })
            })
            .or_else(|| {
                candidates.iter().copied().find(|a| {
                    let l = a.name.to_lowercase();
                    !l.contains("linux")
                        && !l.contains("macos")
                        && (l.ends_with(".exe") || l.ends_with(".zip"))
                })
            }),
        "macos" => {
            let arch_tok = if target_arch == "aarch64" || target_arch == "arm64" {
                "arm64"
            } else {
                "x64"
            };
            candidates
                .iter()
                .copied()
                .find(|a| {
                    let l = a.name.to_lowercase();
                    l.contains("macos") && l.contains(arch_tok)
                })
                .or_else(|| {
                    candidates.iter().copied().find(|a| {
                        let l = a.name.to_lowercase();
                        l.contains("macos") && l.contains("x86_64") && arch_tok == "x64"
                    })
                })
                .or_else(|| {
                    candidates
                        .iter()
                        .copied()
                        .find(|a| a.name.to_lowercase().contains("macos"))
                })
        }
        _ => candidates
            .iter()
            .copied()
            .find(|a| {
                let l = a.name.to_lowercase();
                l.contains("linux") && l.ends_with(".tar.gz")
            })
            .or_else(|| {
                candidates
                    .iter()
                    .copied()
                    .find(|a| a.name.to_lowercase().contains("linux"))
            }),
    }
}

pub fn current_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn current_app_display_version() -> &'static str {
    APP_RELEASE_TAG
}

pub fn normalize_repo_slug(input: &str) -> Result<String, String> {
    let mut s = input.trim();
    for prefix in [
        "https://github.com/",
        "http://github.com/",
        "github.com/",
    ] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest;
            break;
        }
    }
    let s = s.trim_matches('/');
    let s = s.strip_suffix(".git").unwrap_or(s).trim_matches('/');
    let mut parts = s.split('/');
    let owner = parts.next().unwrap_or("").trim();
    let repo = parts.next().unwrap_or("").trim();
    if owner.is_empty() || repo.is_empty() || parts.next().is_some() {
        return Err("Invalid GitHub repository (expected owner/repo or https://github.com/owner/repo)".into());
    }
    Ok(format!("{owner}/{repo}"))
}

pub fn parse_release_version(tag: &str) -> Result<Version, String> {
    let trimmed = tag.trim().trim_start_matches(['v', 'V']);
    for (prefix, pre_tag) in [
        ("Alpha-", "alpha.0"),
        ("alpha-", "alpha.0"),
        ("Beta-", "beta.0"),
        ("beta-", "beta.0"),
        ("RC-", "rc.0"),
        ("rc-", "rc.0"),
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let core = rest.trim_start_matches(['v', 'V']);
            let candidate = if core.contains('-') {
                core.to_string()
            } else {
                format!("{core}-{pre_tag}")
            };
            return Version::parse(&candidate)
                .map_err(|e| format!("Invalid release tag '{tag}': {e}"));
        }
    }
    Version::parse(trimmed).map_err(|e| format!("Invalid release tag '{tag}': {e}"))
}

fn is_legacy_geodesync_release(rel: &GithubRelease) -> bool {
    let name_lower = rel.name.trim().to_lowercase();
    let tag_lower = rel.tag_name.trim().to_lowercase();
    if name_lower.starts_with("geodesync") {
        return true;
    }
    let is_geodeck_prerelease_tag = tag_lower.starts_with("alpha-")
        || tag_lower.starts_with("beta-")
        || tag_lower.starts_with("rc-")
        || tag_lower.contains("geodeck");
    if !is_geodeck_prerelease_tag && name_lower.contains("geodesync") {
        return true;
    }
    false
}

pub fn check_github_updates(repo: &str) -> Result<UpdateCheckResult, String> {
    let slug = normalize_repo_slug(repo)?;
    let url = format!("https://api.github.com/repos/{slug}/releases?per_page=10");
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(6))
        .timeout_read(Duration::from_secs(10))
        .build();

    let resp = agent
        .get(&url)
        .set("User-Agent", &format!("GeoDeck/{}", current_app_display_version()))
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("Update check failed: {e}"))?;

    let releases: Vec<GithubRelease> = resp
        .into_json()
        .map_err(|e| format!("Failed to parse GitHub releases JSON: {e}"))?;

    let current_ver = parse_release_version(APP_RELEASE_TAG)
        .unwrap_or_else(|_| Version::parse("1.0.0-alpha.0").unwrap());

    let mut best: Option<(Version, GithubRelease)> = None;
    for rel in releases {
        if rel.draft || is_legacy_geodesync_release(&rel) {
            continue;
        }
        if let Ok(ver) = parse_release_version(&rel.tag_name) {
            if best.as_ref().is_none_or(|(b_ver, _)| ver > *b_ver) {
                best = Some((ver, rel));
            }
        }
    }

    let Some((latest_ver, rel)) = best else {
        return Ok(UpdateCheckResult {
            current_version: APP_RELEASE_TAG.into(),
            latest_version: APP_RELEASE_TAG.into(),
            update_available: false,
            release_url: format!("https://github.com/{slug}/releases"),
            release_notes: "No newer GeoDeck releases published yet.".into(),
            asset_name: None,
            asset_url: None,
            expected_sha256: None,
        });
    };

    let platform_asset = select_platform_asset_for(
        &rel.assets,
        std::env::consts::OS,
        std::env::consts::ARCH,
    );

    let mut expected_sha256 = platform_asset
        .and_then(|a| a.digest.as_deref())
        .and_then(|d| d.strip_prefix("sha256:").map(|s| s.to_string()));

    if expected_sha256.is_none() {
        if let Some(asset) = platform_asset {
            let sidecar_name = format!("{}.sha256", asset.name).to_lowercase();
            let sha_asset = rel
                .assets
                .iter()
                .find(|a| a.name.to_lowercase() == sidecar_name)
                .or_else(|| {
                    rel.assets
                        .iter()
                        .find(|a| a.name.to_lowercase() == "sha256sums.txt")
                })
                .or_else(|| {
                    rel.assets
                        .iter()
                        .find(|a| a.name.to_lowercase().ends_with(".sha256"))
                });
            if let Some(sha_asset) = sha_asset {
                if let Ok(r) = agent
                    .get(&sha_asset.browser_download_url)
                    .set(
                        "User-Agent",
                        &format!("GeoDeck/{}", current_app_display_version()),
                    )
                    .call()
                {
                    if let Ok(text) = r.into_string() {
                        let target_lower = asset.name.to_lowercase();
                        for line in text.lines() {
                            if text.lines().count() > 1 && !line.to_lowercase().contains(&target_lower) {
                                continue;
                            }
                            if let Some(token) = line.split_whitespace().next() {
                                if token.len() == 64 && token.chars().all(|c| c.is_ascii_hexdigit())
                                {
                                    expected_sha256 = Some(token.to_lowercase());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(UpdateCheckResult {
        current_version: APP_RELEASE_TAG.into(),
        latest_version: rel.tag_name.clone(),
        update_available: latest_ver > current_ver,
        release_url: rel.html_url,
        release_notes: rel.body,
        asset_name: platform_asset.map(|a| a.name.clone()),
        asset_url: platform_asset.map(|a| a.browser_download_url.clone()),
        expected_sha256,
    })
}

pub fn download_and_verify_update(
    asset_url: &str,
    asset_name: &str,
    expected_sha256: Option<&str>,
) -> Result<PathBuf, String> {
    let updates_dir = crate::core::config::config_dir().join("updates");
    fs::create_dir_all(&updates_dir).map_err(|e| format!("Create updates dir: {e}"))?;
    let safe_name: String = asset_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let dest = updates_dir.join(safe_name);

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout_read(Duration::from_secs(30))
        .build();
    let resp = agent
        .get(asset_url)
        .set("User-Agent", &format!("GeoDeck/{}", current_app_display_version()))
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;

    let mut reader = resp.into_reader();
    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .map_err(|e| format!("Read stream failed: {e}"))?;

    let mut hasher = Sha256::new();
    hasher.update(&buf);
    let actual = format!("{:x}", hasher.finalize());

    if let Some(exp) = expected_sha256 {
        if !actual.eq_ignore_ascii_case(exp.trim()) {
            return Err(format!(
                "Update SHA-256 mismatch: expected {}, got {}",
                exp, actual
            ));
        }
    }

    fs::write(&dest, &buf).map_err(|e| format!("Write staged update: {e}"))?;
    Ok(dest)
}
