use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
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
    let url = format!(
        "https://pointercrate.com/api/v2/demons/listed/?limit={}",
        limit.clamp(10, 150)
    );
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout_read(Duration::from_secs(12))
        .build();

    let resp = agent
        .get(&url)
        .set(
            "User-Agent",
            &format!("GeoDeck/{}", crate::core::updater::current_app_display_version()),
        )
        .call()
        .map_err(|e| format!("Pointercrate API failed: {e}"))?;

    let raw = resp
        .into_string()
        .map_err(|e| format!("Failed reading Pointercrate response: {e}"))?;
    parse_pointercrate_json(&raw)
}

/// Cross-references a Pointercrate demon name against the player's local `CCGameManager.dat` progress map.
pub fn match_demon_progress(
    demon_name: &str,
    progress_map: &BTreeMap<String, (u32, u32)>,
) -> Option<(u32, u32)> {
    let key = demon_name.trim().to_lowercase();
    progress_map.get(&key).copied()
}
