use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::geode::package::{DepImportance, InstalledMod};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IssueAction {
    InstallMod(String),
    EnableMod(String),
    SmartRepairMod(String),
    DisableMod(String),
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompatibilityIssue {
    pub severity: IssueSeverity,
    pub mod_id: String,
    pub mod_name: String,
    pub summary: String,
    pub detail: String,
    pub action: IssueAction,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompatibilityAudit {
    pub total_mods: usize,
    pub enabled_mods: usize,
    pub errors: usize,
    pub warnings: usize,
    pub issues: Vec<CompatibilityIssue>,
}

fn clean_semver(raw: &str) -> Option<Version> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    Version::parse(trimmed).ok()
}

fn version_satisfies(installed_ver: &str, req_str: &str) -> bool {
    let req_clean = req_str.trim();
    if req_clean.is_empty() || req_clean == "*" {
        return true;
    }
    let Some(inst_v) = clean_semver(installed_ver) else {
        return true;
    };
    let req_normalized = req_clean
        .replace("v", "")
        .replace("V", "");
    if let Ok(req) = VersionReq::parse(&req_normalized) {
        return req.matches(&inst_v);
    }
    if let Ok(exact) = Version::parse(&req_normalized) {
        return inst_v >= exact;
    }
    true
}

fn gd_target_matches(mod_gd: Option<&str>, active_gd: Option<&str>) -> bool {
    let Some(active) = active_gd else {
        return true;
    };
    let Some(target) = mod_gd else {
        return true;
    };
    let t = target.trim();
    if t.is_empty() || t == "*" {
        return true;
    }
    t == active
}

pub fn audit_mods(
    mods: &[InstalledMod],
    active_gd_version: Option<&str>,
    active_geode_version: Option<&str>,
) -> CompatibilityAudit {
    let mut by_id: HashMap<&str, &InstalledMod> = HashMap::new();
    for m in mods {
        by_id.insert(m.id.as_str(), m);
    }

    let mut issues = Vec::new();
    let loader_ver_norm = active_geode_version
        .map(|s| s.trim_start_matches(['v', 'V']))
        .and_then(|s| {
            let parts: Vec<&str> = s.split('.').collect();
            if parts.len() >= 3 {
                Some(format!("{}.{}.{}", parts[0], parts[1], parts[2]))
            } else {
                Some(s.to_string())
            }
        });

    for m in mods {
        if m.version == "corrupt" {
            issues.push(CompatibilityIssue {
                severity: IssueSeverity::Error,
                mod_id: m.id.clone(),
                mod_name: m.name.clone(),
                summary: "Corrupted .geode archive".into(),
                detail: "Missing or unreadable mod.json inside package.".into(),
                action: IssueAction::DisableMod(m.id.clone()),
            });
            continue;
        }

        if !m.enabled {
            continue;
        }

        // 1. Check Geometry Dash target match
        if !gd_target_matches(m.gd_win_target.as_deref(), active_gd_version) {
            let mod_gd = m.gd_win_target.as_deref().unwrap_or("?");
            let act_gd = active_gd_version.unwrap_or("?");
            issues.push(CompatibilityIssue {
                severity: IssueSeverity::Error,
                mod_id: m.id.clone(),
                mod_name: m.name.clone(),
                summary: format!("Targets GD {mod_gd} (active: {act_gd})"),
                detail: format!(
                    "{} v{} expects Geometry Dash {mod_gd}, while this slot runs {act_gd}.",
                    m.name, m.version
                ),
                action: IssueAction::SmartRepairMod(m.id.clone()),
            });
        }

        // 2. Check Geode loader minimum / major ABI match
        if let Some(ref loader_v) = loader_ver_norm {
            if let (Some(lv), Some(mv)) = (clean_semver(loader_v), clean_semver(&m.geode_target)) {
                if mv.major != lv.major {
                    issues.push(CompatibilityIssue {
                        severity: IssueSeverity::Error,
                        mod_id: m.id.clone(),
                        mod_name: m.name.clone(),
                        summary: format!(
                            "Requires Geode v{} ABI (installed: v{})",
                            m.geode_target, loader_v
                        ),
                        detail: format!(
                            "{} was compiled against Geode v{} and is ABI-incompatible with Geode v{}.",
                            m.name, mv.major, lv.major
                        ),
                        action: IssueAction::SmartRepairMod(m.id.clone()),
                    });
                } else if lv < mv {
                    issues.push(CompatibilityIssue {
                        severity: IssueSeverity::Warning,
                        mod_id: m.id.clone(),
                        mod_name: m.name.clone(),
                        summary: format!("Needs newer Geode >= {}", m.geode_target),
                        detail: format!(
                            "Installed Geode v{} is older than the mod's minimum target v{}.",
                            loader_v, m.geode_target
                        ),
                        action: IssueAction::None,
                    });
                }
            }
        }

        // 3. Check dependencies
        for dep in &m.dependencies {
            if dep.id == "geode.loader" {
                continue;
            }
            match by_id.get(dep.id.as_str()) {
                None => {
                    let sev = match dep.importance {
                        DepImportance::Required => IssueSeverity::Error,
                        DepImportance::Recommended | DepImportance::Suggested => {
                            IssueSeverity::Warning
                        }
                    };
                    let label = match dep.importance {
                        DepImportance::Required => "Missing required dependency",
                        DepImportance::Recommended => "Missing recommended dependency",
                        DepImportance::Suggested => "Missing optional dependency",
                    };
                    issues.push(CompatibilityIssue {
                        severity: sev,
                        mod_id: m.id.clone(),
                        mod_name: m.name.clone(),
                        summary: format!("{label}: {}", dep.id),
                        detail: format!(
                            "{} requires {} ({}).",
                            m.name, dep.id, dep.version
                        ),
                        action: IssueAction::InstallMod(dep.id.clone()),
                    });
                }
                Some(dep_mod) => {
                    if !dep_mod.enabled && dep.importance == DepImportance::Required {
                        issues.push(CompatibilityIssue {
                            severity: IssueSeverity::Error,
                            mod_id: m.id.clone(),
                            mod_name: m.name.clone(),
                            summary: format!("Required dependency disabled: {}", dep.id),
                            detail: format!(
                                "{} needs {} to be enabled.",
                                m.name, dep_mod.name
                            ),
                            action: IssueAction::EnableMod(dep.id.clone()),
                        });
                    } else if !version_satisfies(&dep_mod.version, &dep.version) {
                        issues.push(CompatibilityIssue {
                            severity: IssueSeverity::Warning,
                            mod_id: m.id.clone(),
                            mod_name: m.name.clone(),
                            summary: format!(
                                "Dependency version mismatch: {} ({} vs {})",
                                dep.id, dep_mod.version, dep.version
                            ),
                            detail: format!(
                                "{} expects {} {}, found {}.",
                                m.name, dep.id, dep.version, dep_mod.version
                            ),
                            action: IssueAction::SmartRepairMod(dep.id.clone()),
                        });
                    }
                }
            }
        }

        // 4. Check incompatibilities against other enabled mods
        for inc in &m.incompatibilities {
            if let Some(other) = by_id.get(inc.id.as_str()) {
                if other.enabled && version_satisfies(&other.version, &inc.version) {
                    let sev = if inc.importance == "breaking" {
                        IssueSeverity::Error
                    } else {
                        IssueSeverity::Warning
                    };
                    issues.push(CompatibilityIssue {
                        severity: sev,
                        mod_id: m.id.clone(),
                        mod_name: m.name.clone(),
                        summary: format!("Conflicts with {}", other.name),
                        detail: format!(
                            "{} declares a '{}' incompatibility with {} ({}).",
                            m.name, inc.importance, other.id, inc.version
                        ),
                        action: IssueAction::DisableMod(other.id.clone()),
                    });
                }
            }
        }
    }

    let errors = issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .count();
    let warnings = issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Warning)
        .count();

    CompatibilityAudit {
        total_mods: mods.len(),
        enabled_mods: mods.iter().filter(|m| m.enabled).count(),
        errors,
        warnings,
        issues,
    }
}
