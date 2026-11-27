use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::time::Duration;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PointercrateDemon {
    pub position: u32,
    pub name: String,
    pub requirement: u32,
    pub video: String,
    pub publisher: String,
    pub verifier: String,
}

pub fn pointercrate_page_url(limit: usize, after: Option<u32>) -> String {
    let clamped = limit.clamp(1, 100);
    match after {
        Some(pos) => format!(
            "https://pointercrate.com/api/v2/demons/listed/?limit={clamped}&after={pos}"
        ),
        None => format!("https://pointercrate.com/api/v2/demons/listed/?limit={clamped}"),
    }
}

pub fn parse_pointercrate_json(raw: &str) -> Result<Vec<PointercrateDemon>, String> {
    let arr: Vec<Value> =
        serde_json::from_str(raw).map_err(|e| format!("Invalid Pointercrate JSON: {e}"))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let position = obj.get("position").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let requirement = obj
            .get("requirement")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as u32;
        let video = obj
            .get("video")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let publisher = obj
            .get("publisher")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let verifier = obj
            .get("verifier")
            .and_then(|v| v.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown")
            .to_string();

        out.push(PointercrateDemon {
            position,
            name,
            requirement,
            video,
            publisher,
            verifier,
        });
    }
    out.sort_by_key(|d| d.position);
    Ok(out)
}

pub fn fetch_pointercrate_demons(limit: usize) -> Result<Vec<PointercrateDemon>, String> {
    let target = limit.clamp(10, 150);
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout_read(Duration::from_secs(12))
        .build();
    let user_agent = format!("GeoDeck/{}", crate::core::updater::current_app_display_version());

    let mut all = Vec::with_capacity(target);
    let mut seen_positions = HashSet::new();
    let mut after_pos: Option<u32> = None;

    while all.len() < target {
        let page_limit = (target - all.len()).min(100);
        let url = pointercrate_page_url(page_limit, after_pos);

        let resp = agent
            .get(&url)
            .set("User-Agent", &user_agent)
            .set("Accept", "application/json")
            .call()
            .map_err(|e| format!("Pointercrate API failed: {e}"))?;

        let raw = resp
            .into_string()
            .map_err(|e| format!("Failed reading Pointercrate response: {e}"))?;
        let page = parse_pointercrate_json(&raw)?;
        if page.is_empty() {
            break;
        }

        let last_pos = page.iter().map(|d| d.position).max();
        let page_count = page.len();
        for demon in page {
            if seen_positions.insert(demon.position) {
                all.push(demon);
            }
        }

        if page_count < page_limit || last_pos.is_none() || last_pos == after_pos {
            break;
        }
        after_pos = last_pos;
    }

    all.sort_by_key(|d| d.position);
    all.truncate(target);
    Ok(all)
}

/// Cross-references a Pointercrate demon name against the player's local `CCGameManager.dat` progress map.
pub fn match_demon_progress(
    demon_name: &str,
    progress_map: &BTreeMap<String, (u32, u32)>,
) -> Option<(u32, u32)> {
    let key = demon_name.trim().to_lowercase();
    progress_map.get(&key).copied()
}
