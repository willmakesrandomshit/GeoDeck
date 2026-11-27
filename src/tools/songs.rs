use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use flate2::read::ZlibDecoder;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::backup::save_inspector::SaveSummary;
use crate::core::config::config_dir;
use crate::geode::package::local_gd_data_dir;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MetadataSource {
    Manual,
    JukeboxNong,
    GameManager,
    MusicLibrary,
    SfxLibrary,
    AudioTags,
    BoomlingsApi,
    #[default]
    Unknown,
}

impl MetadataSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Manual => "Custom Name",
            Self::JukeboxNong => "Jukebox NONG",
            Self::GameManager => "GD Save Cache",
            Self::MusicLibrary => "GD Music Library",
            Self::SfxLibrary => "GD SFX Library",
            Self::AudioTags => "Audio Tags",
            Self::BoomlingsApi => "Newgrounds / GD",
            Self::Unknown => "Unresolved",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SongCacheStatus {
    Cached,
    NongActive,
    ManualRenamed,
    Sfx,
    Unknown,
    Broken,
}

impl SongCacheStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cached => "CACHED",
            Self::NongActive => "NONG",
            Self::ManualRenamed => "RENAMED",
            Self::Sfx => "SFX",
            Self::Unknown => "UNKNOWN",
            Self::Broken => "BROKEN",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CachedSongMeta {
    pub title: String,
    pub artist: String,
    pub source: MetadataSource,
    pub manual_override: bool,
    pub unresolved: bool,
    pub updated_at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SongMetadataCache {
    pub schema_version: u32,
    pub songs: BTreeMap<String, CachedSongMeta>,
    pub song_to_levels: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedSong {
    pub id: String,
    pub filename: String,
    pub path: String,
    pub format: String,
    pub size_bytes: u64,
    pub modified: String,
    pub has_nong_backup: bool,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub source: MetadataSource,
    pub manual_override: bool,
    pub jukebox_nong_active: bool,
    pub jukebox_nong_title: Option<String>,
    pub jukebox_nong_artist: Option<String>,
    pub is_sfx: bool,
    pub associated_levels: Vec<String>,
    pub needs_remote_lookup: bool,
}

impl CachedSong {
    pub fn display_title(&self) -> String {
        if let Some(ref t) = self.title {
            if !t.trim().is_empty() {
                return t.clone();
            }
        }
        format!("Unknown Song — ID: {}", self.id)
    }

    pub fn display_artist(&self) -> String {
        if let Some(ref a) = self.artist {
            if !a.trim().is_empty() {
                return a.clone();
            }
        }
        "Metadata unavailable".to_string()
    }

    pub fn is_resolved(&self) -> bool {
        self.title.as_ref().is_some_and(|t| !t.trim().is_empty())
    }

    pub fn status(&self) -> SongCacheStatus {
        if self.size_bytes == 0 {
            SongCacheStatus::Broken
        } else if self.manual_override {
            SongCacheStatus::ManualRenamed
        } else if self.has_nong_backup || self.jukebox_nong_active {
            SongCacheStatus::NongActive
        } else if self.is_sfx {
            SongCacheStatus::Sfx
        } else if self.is_resolved() {
            SongCacheStatus::Cached
        } else {
            SongCacheStatus::Unknown
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SongCacheReport {
    pub total_songs: usize,
    pub total_bytes: u64,
    pub zero_byte_count: usize,
    pub resolved_count: usize,
    pub nong_count: usize,
    pub songs: Vec<CachedSong>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SongSortMode {
    #[default]
    FileSizeDesc,
    TitleAsc,
    TitleDesc,
    ArtistAsc,
    SongIdAsc,
}

impl SongSortMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::FileSizeDesc => "File Size",
            Self::TitleAsc => "Title (A-Z)",
            Self::TitleDesc => "Title (Z-A)",
            Self::ArtistAsc => "Artist (A-Z)",
            Self::SongIdAsc => "Song ID",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SongFormatFilter {
    #[default]
    All,
    Mp3,
    Ogg,
}

impl SongFormatFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All Formats",
            Self::Mp3 => "MP3",
            Self::Ogg => "OGG",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SongStatusFilter {
    #[default]
    All,
    MusicOnly,
    Resolved,
    NongActive,
    ManualRenamed,
    SfxOnly,
    Unknown,
    Broken,
}

impl SongStatusFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All Entries",
            Self::MusicOnly => "Music Tracks",
            Self::Resolved => "Resolved",
            Self::NongActive => "NONG Active",
            Self::ManualRenamed => "Custom Renamed",
            Self::SfxOnly => "GD SFX",
            Self::Unknown => "Unknown",
            Self::Broken => "Broken (0 B)",
        }
    }
}

fn nong_backup_dir() -> PathBuf {
    config_dir().join("nong_backups")
}

pub fn default_song_metadata_cache_path() -> PathBuf {
    config_dir().join("song_metadata_cache.json")
}

pub fn load_metadata_cache_at_path(path: &Path) -> SongMetadataCache {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<SongMetadataCache>(&s).ok())
        .unwrap_or_else(|| SongMetadataCache {
            schema_version: 1,
            ..Default::default()
        })
}

pub fn save_metadata_cache_at_path(path: &Path, cache: &SongMetadataCache) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create cache dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(cache).map_err(|e| format!("serialize cache: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|e| format!("write temp cache: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("commit song cache: {e}"))?;
    Ok(())
}

pub fn merge_save_metadata_into_cache(cache_path: &Path, summary: &SaveSummary) {
    let mut cache = load_metadata_cache_at_path(cache_path);
    cache.schema_version = 1;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut changed = false;

    for entry in &summary.known_songs {
        if entry.song_id.is_empty() || entry.title.trim().is_empty() {
            continue;
        }
        let existing = cache.songs.get(&entry.song_id);
        let skip = existing.is_some_and(|e| e.manual_override || (!e.title.is_empty() && e.source == MetadataSource::GameManager && e.title == entry.title && e.artist == entry.artist));
        if !skip {
            let can_overwrite = existing.is_none_or(|e| !e.manual_override && (e.title.is_empty() || e.unresolved));
            if can_overwrite {
                cache.songs.insert(
                    entry.song_id.clone(),
                    CachedSongMeta {
                        title: entry.title.clone(),
                        artist: entry.artist.clone(),
                        source: MetadataSource::GameManager,
                        manual_override: false,
                        unresolved: false,
                        updated_at: now.clone(),
                    },
                );
                changed = true;
            }
        }
    }

    for (sid, levels) in &summary.song_to_levels {
        if levels.is_empty() {
            continue;
        }
        let existing = cache.song_to_levels.get(sid);
        if existing != Some(levels) {
            cache.song_to_levels.insert(sid.clone(), levels.clone());
            changed = true;
        }
    }

    if changed {
        let _ = save_metadata_cache_at_path(cache_path, &cache);
    }
}

pub fn set_manual_song_metadata(song_id: &str, title: &str, artist: &str) -> Result<(), String> {
    set_manual_song_metadata_at_path(&default_song_metadata_cache_path(), song_id, title, artist)
}

pub fn set_manual_song_metadata_at_path(
    cache_path: &Path,
    song_id: &str,
    title: &str,
    artist: &str,
) -> Result<(), String> {
    let clean_title = title.trim();
    if clean_title.is_empty() {
        return Err("Song title cannot be empty.".into());
    }
    let clean_artist = artist.trim();
    let mut cache = load_metadata_cache_at_path(cache_path);
    cache.schema_version = 1;
    cache.songs.insert(
        song_id.to_string(),
        CachedSongMeta {
            title: clean_title.to_string(),
            artist: if clean_artist.is_empty() {
                "Unknown Artist".into()
            } else {
                clean_artist.to_string()
            },
            source: MetadataSource::Manual,
            manual_override: true,
            unresolved: false,
            updated_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        },
    );
    save_metadata_cache_at_path(cache_path, &cache)
}

pub fn clear_manual_song_metadata(song_id: &str) -> Result<(), String> {
    clear_manual_song_metadata_at_path(&default_song_metadata_cache_path(), song_id)
}

pub fn clear_manual_song_metadata_at_path(cache_path: &Path, song_id: &str) -> Result<(), String> {
    let mut cache = load_metadata_cache_at_path(cache_path);
    if cache.songs.remove(song_id).is_some() {
        save_metadata_cache_at_path(cache_path, &cache)?;
    }
    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct JukeboxSongInfo {
    pub default_title: Option<String>,
    pub default_artist: Option<String>,
    pub nong_active: bool,
    pub nong_title: Option<String>,
    pub nong_artist: Option<String>,
}

#[derive(Deserialize)]
struct JukeboxManifest {
    #[serde(default)]
    default: Option<JukeboxEntry>,
    #[serde(default)]
    active: Option<String>,
    #[serde(default)]
    locals: Vec<JukeboxEntry>,
    #[serde(default)]
    youtube: Vec<JukeboxEntry>,
    #[serde(default)]
    hosted: Vec<JukeboxEntry>,
}

#[derive(Deserialize)]
struct JukeboxEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    artist: String,
    #[serde(default)]
    unique_id: String,
}

pub fn parse_jukebox_manifest_json(raw: &str) -> Option<JukeboxSongInfo> {
    let mf: JukeboxManifest = serde_json::from_str(raw).ok()?;
    let def_uid = mf
        .default
        .as_ref()
        .map(|d| d.unique_id.clone())
        .unwrap_or_default();
    let default_title = mf
        .default
        .as_ref()
        .map(|d| d.name.trim().to_string())
        .filter(|s| !s.is_empty());
    let default_artist = mf
        .default
        .as_ref()
        .map(|d| d.artist.trim().to_string())
        .filter(|s| !s.is_empty());

    let mut nong_active = false;
    let mut nong_title = None;
    let mut nong_artist = None;
    if let Some(ref active_uid) = mf.active {
        if !active_uid.is_empty() && active_uid != &def_uid {
            if let Some(found) = mf
                .hosted
                .iter()
                .chain(mf.locals.iter())
                .chain(mf.youtube.iter())
                .find(|e| e.unique_id == *active_uid)
            {
                nong_active = true;
                let t = found.name.trim().to_string();
                let a = found.artist.trim().to_string();
                if !t.is_empty() {
                    nong_title = Some(t);
                }
                if !a.is_empty() {
                    nong_artist = Some(a);
                }
            }
        }
    }

    Some(JukeboxSongInfo {
        default_title,
        default_artist,
        nong_active,
        nong_title,
        nong_artist,
    })
}

pub fn read_jukebox_manifests(gd_data_dir: &Path) -> HashMap<String, JukeboxSongInfo> {
    let mut out = HashMap::new();
    let mf_dir = gd_data_dir
        .join("geode")
        .join("mods")
        .join("fleym.nongd")
        .join("manifest");
    let Ok(rd) = fs::read_dir(&mf_dir) else {
        return out;
    };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if let Ok(raw) = fs::read_to_string(&p) {
            if let Some(info) = parse_jukebox_manifest_json(&raw) {
                out.insert(stem.to_string(), info);
            }
        }
    }
    out
}

pub fn is_sfx_song_id(id: &str) -> bool {
    id.starts_with('s') && id.len() > 1 && id[1..].chars().all(|c| c.is_ascii_digit())
}

pub fn scan_song_cache() -> SongCacheReport {
    scan_song_cache_with_paths(
        &local_gd_data_dir(),
        &nong_backup_dir(),
        &default_song_metadata_cache_path(),
    )
}

pub fn scan_song_cache_with_paths(
    gd_data_dir: &Path,
    backup_dir: &Path,
    cache_path: &Path,
) -> SongCacheReport {
    let Ok(rd) = fs::read_dir(gd_data_dir) else {
        return SongCacheReport::default();
    };

    let cache = load_metadata_cache_at_path(cache_path);
    let jukebox = read_jukebox_manifests(gd_data_dir);

    let mut songs = Vec::new();
    let mut total_bytes = 0u64;
    let mut zero_byte_count = 0usize;
    let mut resolved_count = 0usize;
    let mut nong_count = 0usize;

    for entry in rd.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let fname = entry.file_name().to_string_lossy().into_owned();
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        if ext != "mp3" && ext != "ogg" {
            continue;
        }

        let id = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let meta = entry.metadata().ok();
        let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        if size_bytes == 0 {
            zero_byte_count += 1;
        }
        total_bytes += size_bytes;

        let modified = meta
            .and_then(|m| m.modified().ok())
            .map(|t| {
                chrono::DateTime::<chrono::Local>::from(t)
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            })
            .unwrap_or_default();

        let has_nong_backup = backup_dir.join(format!("{fname}.orig")).is_file();
        let is_sfx = is_sfx_song_id(&id);
        let jb = jukebox.get(&id);
        let jukebox_nong_active = jb.is_some_and(|j| j.nong_active);
        let jukebox_nong_title = jb.and_then(|j| j.nong_title.clone());
        let jukebox_nong_artist = jb.and_then(|j| j.nong_artist.clone());

        let cached_entry = cache.songs.get(&id);
        let associated_levels = cache.song_to_levels.get(&id).cloned().unwrap_or_default();

        let mut title: Option<String> = None;
        let mut artist: Option<String> = None;
        let mut source = MetadataSource::Unknown;
        let mut manual_override = false;
        let mut marked_unresolved = false;

        if let Some(c) = cached_entry {
            if c.manual_override && !c.title.trim().is_empty() {
                title = Some(c.title.clone());
                artist = (!c.artist.trim().is_empty()).then(|| c.artist.clone());
                source = MetadataSource::Manual;
                manual_override = true;
            }
            marked_unresolved = c.unresolved;
        }

        if title.is_none() && jukebox_nong_active {
            if let Some(ref jt) = jukebox_nong_title {
                title = Some(jt.clone());
                artist = jukebox_nong_artist.clone();
                source = MetadataSource::JukeboxNong;
            }
        }

        if title.is_none() {
            if let Some(c) = cached_entry {
                if !c.title.trim().is_empty() && !c.unresolved {
                    title = Some(c.title.clone());
                    artist = (!c.artist.trim().is_empty()).then(|| c.artist.clone());
                    source = c.source;
                }
            }
        }

        if title.is_none() {
            if let Some(j) = jb {
                if let Some(ref dt) = j.default_title {
                    title = Some(dt.clone());
                    artist = j.default_artist.clone();
                    source = MetadataSource::GameManager;
                }
            }
        }

        // Fallback for non-SFX songs (or tagged files): inspect embedded ID3 / Vorbis tags
        if title.is_none() && size_bytes > 0 {
            if let Some((tag_t, tag_a)) = read_audio_tags(&p) {
                if let Some(t) = tag_t {
                    if !is_suspicious_generic_tag(&t) {
                        title = Some(t);
                        artist = tag_a.filter(|a| !is_suspicious_generic_tag(a));
                        source = MetadataSource::AudioTags;
                    }
                }
            }
        }

        let needs_remote_lookup =
            title.is_none() && size_bytes > 0 && !marked_unresolved && !id.is_empty();

        if title.is_some() {
            resolved_count += 1;
        }
        if has_nong_backup || jukebox_nong_active {
            nong_count += 1;
        }

        songs.push(CachedSong {
            id,
            filename: fname,
            path: p.to_string_lossy().into_owned(),
            format: ext.to_uppercase(),
            size_bytes,
            modified,
            has_nong_backup,
            title,
            artist,
            source,
            manual_override,
            jukebox_nong_active,
            jukebox_nong_title,
            jukebox_nong_artist,
            is_sfx,
            associated_levels,
            needs_remote_lookup,
        });
    }

    songs.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    SongCacheReport {
        total_songs: songs.len(),
        total_bytes,
        zero_byte_count,
        resolved_count,
        nong_count,
        songs,
    }
}

fn is_suspicious_generic_tag(s: &str) -> bool {
    let lower = s.trim().to_lowercase();
    lower.is_empty()
        || lower == "track"
        || lower == "unknown"
        || lower == "untitled"
        || lower == "audio"
}

pub fn apply_metadata_to_report(
    report: &mut SongCacheReport,
    entries: &[(String, CachedSongMeta)],
) {
    let by_id: HashMap<&str, &CachedSongMeta> =
        entries.iter().map(|(id, m)| (id.as_str(), m)).collect();
    for song in &mut report.songs {
        if let Some(meta) = by_id.get(song.id.as_str()) {
            if !song.manual_override && !meta.title.trim().is_empty() && !meta.unresolved {
                song.title = Some(meta.title.clone());
                song.artist = (!meta.artist.trim().is_empty()).then(|| meta.artist.clone());
                song.source = meta.source;
                song.needs_remote_lookup = false;
            } else if meta.unresolved {
                song.needs_remote_lookup = false;
            }
        }
    }
    report.resolved_count = report.songs.iter().filter(|s| s.is_resolved()).count();
}

pub fn filter_and_sort_songs(
    songs: &[CachedSong],
    query: &str,
    format_filter: SongFormatFilter,
    status_filter: SongStatusFilter,
    sort_mode: SongSortMode,
) -> Vec<CachedSong> {
    let q = query.trim().to_lowercase();
    let mut out: Vec<CachedSong> = songs
        .iter()
        .filter(|s| {
            match format_filter {
                SongFormatFilter::All => {}
                SongFormatFilter::Mp3 => {
                    if s.format != "MP3" {
                        return false;
                    }
                }
                SongFormatFilter::Ogg => {
                    if s.format != "OGG" {
                        return false;
                    }
                }
            }

            let st = s.status();
            match status_filter {
                SongStatusFilter::All => {}
                SongStatusFilter::MusicOnly => {
                    if s.is_sfx {
                        return false;
                    }
                }
                SongStatusFilter::Resolved => {
                    if !s.is_resolved() {
                        return false;
                    }
                }
                SongStatusFilter::NongActive => {
                    if st != SongCacheStatus::NongActive {
                        return false;
                    }
                }
                SongStatusFilter::ManualRenamed => {
                    if !s.manual_override {
                        return false;
                    }
                }
                SongStatusFilter::SfxOnly => {
                    if !s.is_sfx {
                        return false;
                    }
                }
                SongStatusFilter::Unknown => {
                    if st != SongCacheStatus::Unknown {
                        return false;
                    }
                }
                SongStatusFilter::Broken => {
                    if st != SongCacheStatus::Broken {
                        return false;
                    }
                }
            }

            if q.is_empty() {
                return true;
            }

            s.id.to_lowercase().contains(&q)
                || s.filename.to_lowercase().contains(&q)
                || s.title
                    .as_ref()
                    .is_some_and(|t| t.to_lowercase().contains(&q))
                || s.artist
                    .as_ref()
                    .is_some_and(|a| a.to_lowercase().contains(&q))
                || s.associated_levels
                    .iter()
                    .any(|lvl| lvl.to_lowercase().contains(&q))
        })
        .cloned()
        .collect();

    match sort_mode {
        SongSortMode::FileSizeDesc => {
            out.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes).then_with(|| a.id.cmp(&b.id)));
        }
        SongSortMode::TitleAsc => {
            out.sort_by(|a, b| {
                a.is_resolved()
                    .cmp(&b.is_resolved())
                    .reverse()
                    .then_with(|| {
                        a.display_title()
                            .to_lowercase()
                            .cmp(&b.display_title().to_lowercase())
                    })
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        SongSortMode::TitleDesc => {
            out.sort_by(|a, b| {
                a.is_resolved()
                    .cmp(&b.is_resolved())
                    .reverse()
                    .then_with(|| {
                        b.display_title()
                            .to_lowercase()
                            .cmp(&a.display_title().to_lowercase())
                    })
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        SongSortMode::ArtistAsc => {
            out.sort_by(|a, b| {
                a.is_resolved()
                    .cmp(&b.is_resolved())
                    .reverse()
                    .then_with(|| {
                        a.display_artist()
                            .to_lowercase()
                            .cmp(&b.display_artist().to_lowercase())
                    })
                    .then_with(|| {
                        a.display_title()
                            .to_lowercase()
                            .cmp(&b.display_title().to_lowercase())
                    })
            });
        }
        SongSortMode::SongIdAsc => {
            out.sort_by(|a, b| {
                let an = a.id.trim_start_matches('s').parse::<u64>().ok();
                let bn = b.id.trim_start_matches('s').parse::<u64>().ok();
                match (an, bn) {
                    (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.id.cmp(&b.id)),
                    _ => a.id.cmp(&b.id),
                }
            });
        }
    }

    out
}

pub fn install_nong_replacement(song_id: &str, custom_audio_path: &Path) -> Result<String, String> {
    let trimmed = song_id.trim();
    if trimmed.is_empty() || !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Enter a valid numeric Newgrounds Song ID (e.g. 123456).".into());
    }
    if !custom_audio_path.is_file() {
        return Err("Selected replacement audio file does not exist.".into());
    }

    let local = local_gd_data_dir();
    fs::create_dir_all(&local).map_err(|e| e.to_string())?;

    // Preserve extension if user picked .ogg and target doesn't exist, otherwise default to .mp3 or existing file ext
    let ext = custom_audio_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .filter(|e| e == "mp3" || e == "ogg")
        .unwrap_or_else(|| "mp3".into());
    let target_name = if local.join(format!("{trimmed}.ogg")).is_file() && ext == "ogg" {
        format!("{trimmed}.ogg")
    } else {
        format!("{trimmed}.mp3")
    };
    let target = local.join(&target_name);

    if target.is_file() {
        let bdir = nong_backup_dir();
        fs::create_dir_all(&bdir).map_err(|e| e.to_string())?;
        let orig_bak = bdir.join(format!("{target_name}.orig"));
        if !orig_bak.exists() {
            fs::copy(&target, &orig_bak)
                .map_err(|e| format!("backup original audio failed: {e}"))?;
        }
    }

    fs::copy(custom_audio_path, &target)
        .map_err(|e| format!("copy NONG audio to {}: {e}", target.display()))?;

    // Read audio tags or clean file stem so NONG replacement has a readable name if not manually overridden
    let cache_path = default_song_metadata_cache_path();
    let mut cache = load_metadata_cache_at_path(&cache_path);
    if !cache.songs.get(trimmed).is_some_and(|m| m.manual_override) {
        if let Some((Some(t), a)) = read_audio_tags(&target) {
            if !is_suspicious_generic_tag(&t) {
                cache.songs.insert(
                    trimmed.to_string(),
                    CachedSongMeta {
                        title: t,
                        artist: a.unwrap_or_else(|| "Custom NONG".into()),
                        source: MetadataSource::AudioTags,
                        manual_override: false,
                        unresolved: false,
                        updated_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    },
                );
                let _ = save_metadata_cache_at_path(&cache_path, &cache);
            }
        }
    }

    Ok(format!("Replaced song {target_name} with custom NONG track."))
}

pub fn restore_original_song(filename: &str) -> Result<String, String> {
    let orig = nong_backup_dir().join(format!("{filename}.orig"));
    if !orig.is_file() {
        return Err("No backed up original audio found for this Song ID.".into());
    }
    let dst = local_gd_data_dir().join(filename);
    fs::copy(&orig, &dst).map_err(|e| format!("restore original song: {e}"))?;
    let _ = fs::remove_file(&orig);

    if let Some(stem) = Path::new(filename).file_stem().and_then(|s| s.to_str()) {
        let cache_path = default_song_metadata_cache_path();
        let mut cache = load_metadata_cache_at_path(&cache_path);
        if let Some(entry) = cache.songs.get(stem) {
            if !entry.manual_override && entry.source == MetadataSource::AudioTags {
                cache.songs.remove(stem);
                let _ = save_metadata_cache_at_path(&cache_path, &cache);
            }
        }
    }

    Ok(format!("Restored original Newgrounds audio for {filename}."))
}

pub fn remove_broken_zero_byte_songs() -> Result<usize, String> {
    let report = scan_song_cache();
    let mut removed = 0usize;
    for song in report.songs {
        if song.size_bytes == 0 && fs::remove_file(&song.path).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

// === Embedded Audio Tag Reader (MP3 ID3v2 / ID3v1 & Ogg Vorbis Comments) ===

pub fn read_audio_tags(path: &Path) -> Option<(Option<String>, Option<String>)> {
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    if len < 16 {
        return None;
    }
    let head_len = len.min(65536) as usize;
    let mut head = vec![0u8; head_len];
    file.read_exact(&mut head).ok()?;

    if head.starts_with(b"ID3") && head.len() >= 10 {
        if let Some(res) = parse_id3v2_bytes(&head) {
            if res.0.is_some() || res.1.is_some() {
                return Some(res);
            }
        }
    }

    if head.starts_with(b"OggS") {
        if let Some(res) = parse_ogg_vorbis_comment_bytes(&head) {
            if res.0.is_some() || res.1.is_some() {
                return Some(res);
            }
        }
    }

    if len >= 128 {
        if file.seek(SeekFrom::End(-128)).is_ok() {
            let mut tail = [0u8; 128];
            if file.read_exact(&mut tail).is_ok() {
                if let Some(res) = parse_id3v1_bytes(&tail) {
                    if res.0.is_some() || res.1.is_some() {
                        return Some(res);
                    }
                }
            }
        }
    }

    None
}

pub fn parse_id3v1_bytes(tail: &[u8]) -> Option<(Option<String>, Option<String>)> {
    if tail.len() < 128 || &tail[0..3] != b"TAG" {
        return None;
    }
    let title = decode_latin1(&tail[3..33]);
    let artist = decode_latin1(&tail[33..63]);
    Some((title, artist))
}

pub fn parse_id3v2_bytes(data: &[u8]) -> Option<(Option<String>, Option<String>)> {
    if data.len() < 10 || &data[0..3] != b"ID3" {
        return None;
    }
    let major = data[3];
    if major < 2 || major > 4 {
        return None;
    }
    let flags = data[5];
    let tag_size = decode_syncsafe(&data[6..10]) as usize;
    let limit = (10 + tag_size).min(data.len());
    let mut pos = 10usize;

    // Skip extended header if bit 6 set in ID3v2.3/2.4
    if major >= 3 && (flags & 0x40) != 0 && pos + 4 <= limit {
        let ext_len = if major == 4 {
            decode_syncsafe(&data[pos..pos + 4]) as usize
        } else {
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize
                + 4
        };
        pos = (pos + ext_len).min(limit);
    }

    let mut title: Option<String> = None;
    let mut artist: Option<String> = None;

    if major == 2 {
        while pos + 6 <= limit {
            let fid = &data[pos..pos + 3];
            if fid[0] == 0 {
                break;
            }
            let fsize = ((data[pos + 3] as usize) << 16)
                | ((data[pos + 4] as usize) << 8)
                | (data[pos + 5] as usize);
            pos += 6;
            if fsize == 0 || pos + fsize > limit {
                break;
            }
            let payload = &data[pos..pos + fsize];
            if fid == b"TT2" && title.is_none() {
                title = decode_id3_text_payload(payload);
            } else if fid == b"TP1" && artist.is_none() {
                artist = decode_id3_text_payload(payload);
            }
            pos += fsize;
        }
    } else {
        while pos + 10 <= limit {
            let fid = &data[pos..pos + 4];
            if fid[0] == 0 {
                break;
            }
            let fsize = if major == 4 {
                decode_syncsafe(&data[pos + 4..pos + 8]) as usize
            } else {
                u32::from_be_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                    as usize
            };
            pos += 10;
            if fsize == 0 || pos + fsize > limit {
                break;
            }
            let payload = &data[pos..pos + fsize];
            if fid == b"TIT2" && title.is_none() {
                title = decode_id3_text_payload(payload);
            } else if fid == b"TPE1" && artist.is_none() {
                artist = decode_id3_text_payload(payload);
            }
            pos += fsize;
        }
    }

    if title.is_none() && artist.is_none() {
        None
    } else {
        Some((title, artist))
    }
}

fn decode_syncsafe(b: &[u8]) -> u32 {
    ((b[0] as u32 & 0x7F) << 21)
        | ((b[1] as u32 & 0x7F) << 14)
        | ((b[2] as u32 & 0x7F) << 7)
        | (b[3] as u32 & 0x7F)
}

fn decode_latin1(bytes: &[u8]) -> Option<String> {
    let trimmed = bytes.split(|&b| b == 0).next().unwrap_or(&[]);
    let s: String = trimmed.iter().map(|&b| b as char).collect();
    let clean = s.trim().to_string();
    if clean.is_empty() {
        None
    } else {
        Some(clean)
    }
}

fn decode_id3_text_payload(payload: &[u8]) -> Option<String> {
    if payload.is_empty() {
        return None;
    }
    let enc = payload[0];
    let raw = &payload[1..];
    let s = match enc {
        0 => raw
            .split(|&b| b == 0)
            .next()
            .unwrap_or(&[])
            .iter()
            .map(|&b| b as char)
            .collect::<String>(),
        1 => decode_utf16_with_bom(raw),
        2 => decode_utf16_be(raw),
        3 => String::from_utf8_lossy(raw.split(|&b| b == 0).next().unwrap_or(&[])).into_owned(),
        _ => String::new(),
    };
    let clean = s.trim_matches(char::from(0)).trim().to_string();
    if clean.is_empty() {
        None
    } else {
        Some(clean)
    }
}

fn decode_utf16_with_bom(raw: &[u8]) -> String {
    if raw.len() < 2 {
        return String::new();
    }
    let (le, body) = if raw[0] == 0xFF && raw[1] == 0xFE {
        (true, &raw[2..])
    } else if raw[0] == 0xFE && raw[1] == 0xFF {
        (false, &raw[2..])
    } else {
        (true, raw)
    };
    let mut u16s = Vec::with_capacity(body.len() / 2);
    for chunk in body.chunks_exact(2) {
        let code = if le {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        };
        if code == 0 {
            break;
        }
        u16s.push(code);
    }
    String::from_utf16_lossy(&u16s)
}

fn decode_utf16_be(raw: &[u8]) -> String {
    let mut u16s = Vec::with_capacity(raw.len() / 2);
    for chunk in raw.chunks_exact(2) {
        let code = u16::from_be_bytes([chunk[0], chunk[1]]);
        if code == 0 {
            break;
        }
        u16s.push(code);
    }
    String::from_utf16_lossy(&u16s)
}

pub fn parse_ogg_vorbis_comment_bytes(data: &[u8]) -> Option<(Option<String>, Option<String>)> {
    let marker = b"\x03vorbis";
    let pos = data.windows(marker.len()).position(|w| w == marker)?;
    let mut cursor = pos + marker.len();
    if cursor + 4 > data.len() {
        return None;
    }
    let vendor_len = u32::from_le_bytes([
        data[cursor],
        data[cursor + 1],
        data[cursor + 2],
        data[cursor + 3],
    ]) as usize;
    cursor += 4 + vendor_len;
    if cursor + 4 > data.len() {
        return None;
    }
    let count = u32::from_le_bytes([
        data[cursor],
        data[cursor + 1],
        data[cursor + 2],
        data[cursor + 3],
    ]) as usize;
    cursor += 4;

    let mut title = None;
    let mut artist = None;
    for _ in 0..count.min(64) {
        if cursor + 4 > data.len() {
            break;
        }
        let len = u32::from_le_bytes([
            data[cursor],
            data[cursor + 1],
            data[cursor + 2],
            data[cursor + 3],
        ]) as usize;
        cursor += 4;
        if cursor + len > data.len() {
            break;
        }
        if let Ok(entry) = std::str::from_utf8(&data[cursor..cursor + len]) {
            if let Some((k, v)) = entry.split_once('=') {
                let kl = k.trim().to_ascii_uppercase();
                let val = v.trim().to_string();
                if !val.is_empty() {
                    if kl == "TITLE" && title.is_none() {
                        title = Some(val);
                    } else if kl == "ARTIST" && artist.is_none() {
                        artist = Some(val);
                    }
                }
            }
        }
        cursor += len;
    }

    if title.is_none() && artist.is_none() {
        None
    } else {
        Some((title, artist))
    }
}

// === Official GD Metadata Parsers & Async Remote Resolver ===

pub fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub fn parse_boomlings_song_info(body: &str) -> Option<(String, String)> {
    let trimmed = body.trim();
    if trimmed.is_empty() || trimmed == "-1" || trimmed == "-2" {
        return None;
    }
    let parts: Vec<&str> = trimmed.split("~|~").collect();
    let mut title = String::new();
    let mut artist = String::new();
    let mut i = 0usize;
    while i + 1 < parts.len() {
        match parts[i] {
            "2" => title = url_decode(parts[i + 1]).trim().to_string(),
            "4" => artist = url_decode(parts[i + 1]).trim().to_string(),
            _ => {}
        }
        i += 2;
    }
    if title.is_empty() {
        None
    } else {
        if artist.is_empty() {
            artist = "Unknown Artist".into();
        }
        Some((title, artist))
    }
}

pub fn decode_gd_zlib_b64(raw: &[u8]) -> Result<String, String> {
    let mut cleaned: Vec<u8> = raw
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    while cleaned.len() % 4 != 0 {
        cleaned.push(b'=');
    }
    let compressed = URL_SAFE
        .decode(&cleaned)
        .map_err(|e| format!("base64 decode: {e}"))?;
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut out = String::new();
    decoder
        .read_to_string(&mut out)
        .map_err(|e| format!("zlib inflate: {e}"))?;
    Ok(out)
}

pub fn parse_gd_music_library(decoded: &str) -> HashMap<String, (String, String)> {
    let sections: Vec<&str> = decoded.split('|').collect();
    let mut artists: HashMap<String, String> = HashMap::new();
    if let Some(artist_sec) = sections.get(1) {
        for item in artist_sec.split(';') {
            let mut cols = item.split(',');
            if let (Some(aid), Some(name)) = (cols.next(), cols.next()) {
                let a = aid.trim();
                let n = url_decode(name).trim().to_string();
                if !a.is_empty() && !n.is_empty() {
                    artists.insert(a.to_string(), n);
                }
            }
        }
    }

    let mut songs = HashMap::new();
    if let Some(song_sec) = sections.get(2) {
        for item in song_sec.split(';') {
            let cols: Vec<&str> = item.split(',').collect();
            if cols.len() >= 3 {
                let sid = cols[0].trim();
                let title = url_decode(cols[1]).trim().to_string();
                let aid = cols[2].trim();
                if !sid.is_empty() && !title.is_empty() {
                    let artist = artists
                        .get(aid)
                        .cloned()
                        .unwrap_or_else(|| "GD Music Library".into());
                    songs.insert(sid.to_string(), (title, artist));
                }
            }
        }
    }
    songs
}

pub fn parse_gd_sfx_library(decoded: &str) -> HashMap<String, (String, String)> {
    let mut folders: HashMap<String, String> = HashMap::new();
    let mut sfx = HashMap::new();
    let first_sec = decoded.split('|').next().unwrap_or("");

    for item in first_sec.split(';') {
        let cols: Vec<&str> = item.split(',').collect();
        if cols.len() >= 4 {
            let id = cols[0].trim();
            let name = url_decode(cols[1]).trim().to_string();
            let is_folder = cols[2].trim() == "1";
            if is_folder && !id.is_empty() && !name.is_empty() {
                folders.insert(id.to_string(), name);
            }
        }
    }

    for item in first_sec.split(';') {
        let cols: Vec<&str> = item.split(',').collect();
        if cols.len() >= 4 {
            let id = cols[0].trim();
            let name = url_decode(cols[1]).trim().to_string();
            let is_folder = cols[2].trim() == "1";
            let parent = cols[3].trim();
            if !is_folder && !id.is_empty() && !name.is_empty() {
                let category = folders
                    .get(parent)
                    .map(|f| format!("GD SFX · {f}"))
                    .unwrap_or_else(|| "Geometry Dash SFX".into());
                sfx.insert(format!("s{id}"), (name, category));
            }
        }
    }
    sfx
}

pub fn resolve_missing_song_metadata<F>(
    song_ids: &[String],
    force_retry_unresolved: bool,
    on_batch: F,
) -> (usize, bool)
where
    F: Fn(Vec<(String, CachedSongMeta)>),
{
    let cache_path = default_song_metadata_cache_path();
    let mut cache = load_metadata_cache_at_path(&cache_path);
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let mut targets: Vec<String> = Vec::new();
    for id in song_ids {
        if targets.contains(id) {
            continue;
        }
        if let Some(existing) = cache.songs.get(id) {
            if existing.manual_override {
                continue;
            }
            if !existing.title.trim().is_empty() && !existing.unresolved {
                continue;
            }
            if existing.unresolved && !force_retry_unresolved {
                continue;
            }
        }
        targets.push(id.clone());
    }

    if targets.is_empty() {
        return (0, false);
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(6))
        .timeout_read(Duration::from_secs(10))
        .build();

    let mut resolved_total = 0usize;
    let mut network_failures = 0usize;

    // Tier A: GD 2.2 Music Library for IDs >= 10_000_000
    let need_music_lib = targets
        .iter()
        .any(|id| id.parse::<u64>().is_ok_and(|n| n >= 10_000_000));
    if need_music_lib {
        match agent
            .get("https://geometrydashfiles.b-cdn.net/music/musiclibrary_02.dat")
            .set("User-Agent", "")
            .call()
        {
            Ok(resp) => {
                let mut bytes = Vec::new();
                if resp.into_reader().read_to_end(&mut bytes).is_ok() {
                    if let Ok(decoded) = decode_gd_zlib_b64(&bytes) {
                        let lib = parse_gd_music_library(&decoded);
                        let mut batch = Vec::new();
                        for id in &targets {
                            if let Some((t, a)) = lib.get(id) {
                                let meta = CachedSongMeta {
                                    title: t.clone(),
                                    artist: a.clone(),
                                    source: MetadataSource::MusicLibrary,
                                    manual_override: false,
                                    unresolved: false,
                                    updated_at: now.clone(),
                                };
                                cache.songs.insert(id.clone(), meta.clone());
                                batch.push((id.clone(), meta));
                                resolved_total += 1;
                            }
                        }
                        if !batch.is_empty() {
                            let _ = save_metadata_cache_at_path(&cache_path, &cache);
                            on_batch(batch);
                        }
                    }
                }
            }
            Err(_) => {
                network_failures += 1;
            }
        }
    }

    // Tier B: GD 2.2 SFX Library for `s<digits>`
    let need_sfx_lib = targets.iter().any(|id| is_sfx_song_id(id));
    if need_sfx_lib {
        match agent
            .get("https://geometrydashfiles.b-cdn.net/sfx/sfxlibrary.dat")
            .set("User-Agent", "")
            .call()
        {
            Ok(resp) => {
                let mut bytes = Vec::new();
                if resp.into_reader().read_to_end(&mut bytes).is_ok() {
                    if let Ok(decoded) = decode_gd_zlib_b64(&bytes) {
                        let lib = parse_gd_sfx_library(&decoded);
                        let mut batch = Vec::new();
                        for id in &targets {
                            if let Some((t, a)) = lib.get(id) {
                                let meta = CachedSongMeta {
                                    title: t.clone(),
                                    artist: a.clone(),
                                    source: MetadataSource::SfxLibrary,
                                    manual_override: false,
                                    unresolved: false,
                                    updated_at: now.clone(),
                                };
                                cache.songs.insert(id.clone(), meta.clone());
                                batch.push((id.clone(), meta));
                                resolved_total += 1;
                            }
                        }
                        if !batch.is_empty() {
                            let _ = save_metadata_cache_at_path(&cache_path, &cache);
                            on_batch(batch);
                        }
                    }
                }
            }
            Err(_) => {
                network_failures += 1;
            }
        }
    }

    // Tier C: Official Boomlings song info endpoint for remaining numeric Newgrounds IDs (< 10M)
    let remaining_ng: Vec<String> = targets
        .into_iter()
        .filter(|id| {
            !cache.songs.get(id).is_some_and(|m| !m.title.is_empty() && !m.unresolved)
                && id.parse::<u64>().is_ok_and(|n| n > 0 && n < 10_000_000)
        })
        .collect();

    let mut pending_batch: Vec<(String, CachedSongMeta)> = Vec::new();
    let mut consecutive_net_err = 0usize;
    let mut offline_aborted = false;

    for id in remaining_ng {
        if consecutive_net_err >= 3 {
            offline_aborted = true;
            break;
        }
        let body = format!("songID={id}&secret=Wmfd2893gb7");
        let resp = agent
            .post("https://www.boomlings.com/database/getGJSongInfo.php")
            .set("User-Agent", "")
            .set("Content-Type", "application/x-www-form-urlencoded")
            .send_string(&body);

        match resp {
            Ok(r) => {
                consecutive_net_err = 0;
                if let Ok(text) = r.into_string() {
                    if let Some((title, artist)) = parse_boomlings_song_info(&text) {
                        let meta = CachedSongMeta {
                            title,
                            artist,
                            source: MetadataSource::BoomlingsApi,
                            manual_override: false,
                            unresolved: false,
                            updated_at: now.clone(),
                        };
                        cache.songs.insert(id.clone(), meta.clone());
                        pending_batch.push((id, meta));
                        resolved_total += 1;
                    } else if text.trim() == "-1" || text.trim() == "-2" {
                        let meta = CachedSongMeta {
                            title: String::new(),
                            artist: String::new(),
                            source: MetadataSource::Unknown,
                            manual_override: false,
                            unresolved: true,
                            updated_at: now.clone(),
                        };
                        cache.songs.insert(id.clone(), meta.clone());
                        pending_batch.push((id, meta));
                    }
                }
            }
            Err(_) => {
                consecutive_net_err += 1;
                network_failures += 1;
            }
        }

        if pending_batch.len() >= 4 {
            let _ = save_metadata_cache_at_path(&cache_path, &cache);
            on_batch(std::mem::take(&mut pending_batch));
        }

        std::thread::sleep(Duration::from_millis(110));
    }

    if !pending_batch.is_empty() {
        let _ = save_metadata_cache_at_path(&cache_path, &cache);
        on_batch(pending_batch);
    }

    (resolved_total, offline_aborted || (resolved_total == 0 && network_failures > 0))
}
