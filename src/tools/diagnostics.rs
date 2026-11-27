use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::core::config::Config;
use crate::core::platform::{
    current_username, gd_resources_dir, is_steam_library_game_path,
};
#[cfg(target_os = "macos")]
use crate::core::platform::{find_macos_game_app, validate_macos_game_app};
use crate::geode::package::InstalledMod;
use crate::launcher::detection::{find_all_gd_installations, steam_roots, InstallationProbe};
use crate::launcher::integrity::IntegrityReport;
use crate::launcher::versions::is_manifest_locked;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Ok,
    Warning,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiagnosticFinding {
    pub severity: DiagnosticSeverity,
    pub title: String,
    pub detail: String,
    pub steps: String,
}

fn crc32_update(mut crc: u32, data: &[u8]) -> u32 {
    crc = !crc;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Validates PNG signature, IHDR, every chunk's IEEE CRC-32, IDAT presence, and IEND termination.
pub fn inspect_png_resource(path: &Path) -> Option<String> {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let Ok(meta) = fs::metadata(path) else {
        return Some("file could not be accessed".into());
    };
    let file_size = meta.len();
    if file_size < 45 {
        return Some("file is too small to contain a complete PNG".into());
    }
    let Ok(mut file) = File::open(path) else {
        return Some("file could not be read (permission or I/O error)".into());
    };
    let mut sig_buf = [0u8; 8];
    if file.read_exact(&mut sig_buf).is_err() || sig_buf != SIG {
        return Some("PNG signature is missing".into());
    }

    let mut saw_header = false;
    let mut saw_idat = false;
    let mut saw_end = false;
    let mut chunk_buf = Vec::new();

    loop {
        let Ok(pos) = file.stream_position() else {
            return Some("I/O error reading PNG stream".into());
        };
        if pos >= file_size {
            break;
        }
        let mut hdr = [0u8; 8];
        if file.read_exact(&mut hdr).is_err() {
            return Some("truncated PNG chunk header".into());
        }
        let length = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]) as u64;
        let chunk_type = [hdr[4], hdr[5], hdr[6], hdr[7]];
        let Ok(after_hdr) = file.stream_position() else {
            return Some("I/O error".into());
        };
        if length + 4 > file_size.saturating_sub(after_hdr) {
            return Some("PNG chunk extends beyond end of file".into());
        }
        if !saw_header {
            if &chunk_type != b"IHDR" || length != 13 {
                return Some("PNG header chunk is missing or malformed".into());
            }
            saw_header = true;
        }
        chunk_buf.resize(length as usize, 0);
        if length > 0 && file.read_exact(&mut chunk_buf).is_err() {
            return Some("truncated PNG image data".into());
        }
        let mut crc_bytes = [0u8; 4];
        if file.read_exact(&mut crc_bytes).is_err() {
            return Some("truncated PNG chunk checksum".into());
        }
        let stored_crc = u32::from_be_bytes(crc_bytes);
        let mut crc = crc32_update(0, &chunk_type);
        if length > 0 {
            crc = crc32_update(crc, &chunk_buf);
        }
        if crc != stored_crc {
            let tag = String::from_utf8_lossy(&chunk_type);
            return Some(format!("{tag} chunk checksum failed"));
        }
        if &chunk_type == b"IDAT" {
            saw_idat = true;
        }
        if &chunk_type == b"IEND" {
            if length != 0 || !saw_idat {
                return Some("PNG ended before complete image data".into());
            }
            saw_end = true;
            let end_pos = file.stream_position().unwrap_or(file_size);
            if end_pos != file_size {
                return Some("unexpected data follows the PNG end chunk".into());
            }
            break;
        }
    }

    if !saw_header || !saw_idat || !saw_end {
        return Some("PNG is incomplete".into());
    }
    None
}

/// Scans `Resources/*.png` for truncation or CRC corruption and compares against `Resources_Vanilla_Backup`.
pub fn scan_texture_resources(game_path: &str) -> Vec<DiagnosticFinding> {
    if game_path.trim().is_empty() || !Path::new(game_path).is_dir() {
        return Vec::new();
    }
    let root = Path::new(game_path);
    let resources = gd_resources_dir(root);
    let backup = root.join("Resources_Vanilla_Backup");

    if !resources.is_dir() {
        return vec![DiagnosticFinding {
            severity: DiagnosticSeverity::Error,
            title: "Geometry Dash Resources folder is missing".into(),
            detail: "The game's Resources folder is not present, so texture sheets cannot load."
                .into(),
            steps:
                "Close the game and use Steam -> Verify integrity of game files, then rescan."
                    .into(),
        }];
    }

    let Ok(rd) = fs::read_dir(&resources) else {
        return vec![DiagnosticFinding {
            severity: DiagnosticSeverity::Error,
            title: "Geometry Dash textures could not be checked".into(),
            detail: "The Resources folder could not be read due to permissions or cloud lock.".into(),
            steps: "Close Geometry Dash and Steam, verify the folder path, then retry.".into(),
        }];
    };

    let mut current_pngs = Vec::new();
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.to_lowercase().ends_with(".png") {
                current_pngs.push(name);
            }
        }
    }
    current_pngs.sort();

    if current_pngs.is_empty() {
        return vec![DiagnosticFinding {
            severity: DiagnosticSeverity::Warning,
            title: "Geometry Dash has no top-level PNG texture resources".into(),
            detail: "The Resources folder exists but contains no top-level .png spritesheets."
                .into(),
            steps: "Verify Geometry Dash through Steam if textures fail to render.".into(),
        }];
    }

    let mut invalid = Vec::new();
    for fname in &current_pngs {
        if let Some(reason) = inspect_png_resource(&resources.join(fname)) {
            invalid.push(format!("{fname}: {reason}"));
            if invalid.len() >= 5 {
                break;
            }
        }
    }

    let mut missing_from_bak = Vec::new();
    if let Ok(bak_rd) = fs::read_dir(&backup) {
        for entry in bak_rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.to_lowercase().ends_with(".png")
                && !current_pngs.iter().any(|c| c.eq_ignore_ascii_case(&name))
            {
                missing_from_bak.push(name);
                if missing_from_bak.len() >= 8 {
                    break;
                }
            }
        }
    }

    if !invalid.is_empty() || !missing_from_bak.is_empty() {
        let mut parts = Vec::new();
        if !invalid.is_empty() {
            parts.push(format!("PNG integrity check failed for {}", invalid.join("; ")));
        }
        if !missing_from_bak.is_empty() {
            parts.push(format!(
                "Missing compared with vanilla backup: {}",
                missing_from_bak.join(", ")
            ));
        }
        return vec![DiagnosticFinding {
            severity: DiagnosticSeverity::Error,
            title: "Geometry Dash texture files are damaged or missing".into(),
            detail: parts.join(". "),
            steps: "Use Tools -> Texture Packs -> Restore Vanilla Textures, or verify files in Steam.".into(),
        }];
    }

    let active_pack = fs::read_to_string(root.join("geode").join("active_texture_pack.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()));

    if let Some(pack) = active_pack {
        vec![DiagnosticFinding {
            severity: DiagnosticSeverity::Ok,
            title: format!("Custom Texture Pack Active: {pack}"),
            detail: format!(
                "Checked {} top-level PNG spritesheets; no truncation or PNG CRC-32 errors found.",
                current_pngs.len()
            ),
            steps: "If textures appear pink or black in-game, restore vanilla textures in Tools -> Texture Packs.".into(),
        }]
    } else {
        vec![DiagnosticFinding {
            severity: DiagnosticSeverity::Ok,
            title: "Texture PNG integrity verified".into(),
            detail: format!(
                "Scanned {} top-level PNG files with zero CRC-32 checksum or truncation errors.",
                current_pngs.len()
            ),
            steps: "No action needed.".into(),
        }]
    }
}

fn pe_machine_label(machine: u16) -> Option<&'static str> {
    match machine {
        0x014C => Some("x86"),
        0x8664 => Some("x64"),
        0xAA64 => Some("ARM64"),
        _ => None,
    }
}

/// Structurally validates a Windows PE EXE/DLL and returns its COFF `Machine` identifier.
pub fn read_windows_pe_machine(path: &Path) -> Result<u16, String> {
    let file_size = fs::metadata(path)
        .map_err(|e| format!("file cannot be read ({e})"))?
        .len();
    if file_size < 0x40 {
        return Err("file is too small to contain a PE header".into());
    }
    let mut stream = File::open(path).map_err(|e| format!("file cannot be read ({e})"))?;
    let mut dos = [0u8; 0x40];
    stream
        .read_exact(&mut dos)
        .map_err(|e| format!("read DOS header: {e}"))?;
    if &dos[0..2] != b"MZ" {
        return Err("missing DOS MZ signature".into());
    }
    let pe_offset = u32::from_le_bytes([dos[0x3C], dos[0x3D], dos[0x3E], dos[0x3F]]) as u64;
    if pe_offset < 0x40 || pe_offset + 24 > file_size {
        return Err("PE header offset is outside the file".into());
    }
    stream
        .seek(SeekFrom::Start(pe_offset))
        .map_err(|e| format!("seek PE: {e}"))?;
    let mut sig_coff = [0u8; 24];
    stream
        .read_exact(&mut sig_coff)
        .map_err(|e| format!("missing or truncated PE/COFF header: {e}"))?;
    if &sig_coff[0..4] != b"PE\0\0" {
        return Err("missing PE\\0\\0 signature".into());
    }
    let machine = u16::from_le_bytes([sig_coff[4], sig_coff[5]]);
    let section_count = u16::from_le_bytes([sig_coff[6], sig_coff[7]]);
    let optional_size = u16::from_le_bytes([sig_coff[20], sig_coff[21]]);

    if pe_machine_label(machine).is_none() {
        return Err(format!("unsupported PE machine type 0x{machine:04x}"));
    }
    if section_count == 0 || section_count > 96 {
        return Err("invalid PE section count".into());
    }
    if optional_size < 2
        || pe_offset + 24 + (optional_size as u64) + (section_count as u64) * 40 > file_size
    {
        return Err("truncated PE optional header or section table".into());
    }
    let mut opt_buf = vec![0u8; optional_size as usize];
    stream
        .read_exact(&mut opt_buf)
        .map_err(|e| format!("read optional header: {e}"))?;
    let opt_magic = u16::from_le_bytes([opt_buf[0], opt_buf[1]]);
    let expected_magic = if machine == 0x014C { 0x10B } else { 0x20B };
    if opt_magic != expected_magic {
        return Err("PE machine type does not match optional-header format".into());
    }
    let mut sec = [0u8; 40];
    for _ in 0..section_count {
        stream
            .read_exact(&mut sec)
            .map_err(|e| format!("truncated PE section table: {e}"))?;
        let raw_size = u32::from_le_bytes([sec[16], sec[17], sec[18], sec[19]]) as u64;
        let raw_offset = u32::from_le_bytes([sec[20], sec[21], sec[22], sec[23]]) as u64;
        if raw_size > 0 && (raw_offset > file_size || raw_size > file_size - raw_offset) {
            return Err("PE section data extends beyond the file".into());
        }
    }
    Ok(machine)
}

/// Validates that `GeometryDash.exe` and every `.dll` in the folder share the same PE architecture.
pub fn validate_windows_game_images(game_root: &Path) -> Option<String> {
    let exe = game_root.join("GeometryDash.exe");
    if !exe.is_file() {
        return Some("GeometryDash.exe is missing.".into());
    }
    let expected_machine = match read_windows_pe_machine(&exe) {
        Ok(m) => m,
        Err(e) => {
            return Some(format!(
                "GeometryDash.exe is not a valid Windows executable ({e})."
            ))
        }
    };

    let mut stack = vec![game_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                if !name.starts_with(".geodesync-transaction-") {
                    stack.push(p);
                }
            } else if name.to_lowercase().ends_with(".dll") {
                let rel = p
                    .strip_prefix(game_root)
                    .map(|r| r.to_string_lossy().into_owned())
                    .unwrap_or(name);
                match read_windows_pe_machine(&p) {
                    Ok(m) if m != expected_machine => {
                        let actual = pe_machine_label(m).unwrap_or("unknown");
                        let expected = pe_machine_label(expected_machine).unwrap_or("unknown");
                        return Some(format!(
                            "DLL '{rel}' is {actual}, but GeometryDash.exe is {expected}."
                        ));
                    }
                    Err(e) => {
                        return Some(format!(
                            "DLL '{rel}' is damaged or not a valid Windows library ({e})."
                        ));
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

/// Pins OneDrive cloud-only stub files locally on Windows (`attrib.exe -U -P /s /d`).
pub fn ensure_local_onedrive_files(folder: &Path) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        if !folder.is_dir() {
            return Err("Folder does not exist.".into());
        }
        let pattern = folder.join("*");
        let out = std::process::Command::new("attrib.exe")
            .args(["-U", "-P", "/s", "/d"])
            .arg(&pattern)
            .output()
            .map_err(|e| format!("Failed invoking attrib.exe: {e}"))?;
        if out.status.success() {
            Ok("Pinned OneDrive files to local storage (-U -P).".into())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = folder;
        Ok("OneDrive local pinning applies only to Windows.".into())
    }
}

/// Checks Microsoft Visual C++ 2015-2022 Redistributable (x86) status on Windows.
pub fn check_vc_redist_installed() -> (bool, String) {
    #[cfg(not(target_os = "windows"))]
    {
        (true, "Not required on this operating system".into())
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(out) = std::process::Command::new("reg")
            .args([
                "query",
                r"HKLM\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\X86",
                "/reg:32",
            ])
            .output()
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                let mut ver_str = String::new();
                for line in text.lines() {
                    if line.contains("Version") && line.contains("REG_SZ") {
                        if let Some((_, rhs)) = line.split_once("REG_SZ") {
                            ver_str = rhs.trim().to_string();
                        }
                    }
                }
                if text.contains("Installed") && text.contains("0x1") {
                    return (
                        true,
                        if ver_str.is_empty() {
                            "Installed".into()
                        } else {
                            format!("Installed ({ver_str})")
                        },
                    );
                }
            }
        }
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        let syswow = Path::new(&windir).join("SysWOW64").join("msvcp140.dll");
        let sys32 = Path::new(&windir).join("System32").join("msvcp140.dll");
        if syswow.is_file() || sys32.is_file() {
            (true, "Detected via system msvcp140.dll".into())
        } else {
            (false, "Missing or outdated (MSVCP140/VCRUNTIME140)".into())
        }
    }
}

/// Checks whether a directory is writable and has at least 500 MB free space.
pub fn check_folder_writable_and_space(folder: &Path) -> (bool, String, u64) {
    if !folder.is_dir() {
        return (false, "Directory does not exist".into(), 0);
    }

    let test_file = folder.join(format!(".geodeck-write-test-{}.tmp", std::process::id()));
    if let Err(e) = fs::write(&test_file, b"ok") {
        let lower = e.to_string().to_lowercase();
        let msg = if lower.contains("112") || lower.contains("space") || lower.contains("quota") {
            "Drive or OneDrive storage quota is full; cannot write files.".into()
        } else {
            format!("Directory is not writable: {e}")
        };
        return (false, msg, 0);
    }
    let _ = fs::remove_file(&test_file);

    let disks = sysinfo::Disks::new_with_refreshed_list();
    let canon = fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
    let mut best_mount_len = 0usize;
    let mut free_mb = 2048u64;
    for disk in disks.list() {
        let mp = disk.mount_point();
        if canon.starts_with(mp) {
            let len = mp.to_string_lossy().len();
            if len >= best_mount_len {
                best_mount_len = len;
                free_mb = disk.available_space() / (1024 * 1024);
            }
        }
    }

    if free_mb < 500 {
        return (
            false,
            format!("Critically low disk space ({free_mb} MB remaining). At least 500 MB required."),
            free_mb,
        );
    }
    (true, "Writable with adequate storage".into(), free_mb)
}

/// Runs all 8 installation health checks (read-only except temporary write probe) without modifying game files.
pub fn scan_installation_issues(game_path: &str, save_dir: &Path) -> Vec<DiagnosticFinding> {
    let mut findings = Vec::new();
    let roots = steam_roots();
    if roots.is_empty() {
        findings.push(DiagnosticFinding {
            severity: DiagnosticSeverity::Warning,
            title: "Steam library was not auto-detected".into(),
            detail: "GeoDeck could not find a standard Steam installation directory.".into(),
            steps: "Open Steam and sign in, or add your Geometry Dash folder manually in Installs.".into(),
        });
    }

    if game_path.trim().is_empty() || !Path::new(game_path).is_dir() {
        findings.push(DiagnosticFinding {
            severity: DiagnosticSeverity::Error,
            title: "Geometry Dash folder not found".into(),
            detail: "The active game installation path is missing or not selected.".into(),
            steps: "Go to Installs -> Add Install Folder and select your Geometry Dash directory.".into(),
        });
        return findings;
    }

    let root = Path::new(game_path);

    if is_steam_library_game_path(game_path) {
        findings.push(DiagnosticFinding {
            severity: DiagnosticSeverity::Ok,
            title: "Steam Geometry Dash layout confirmed".into(),
            detail: format!("Selected path follows Steam's steamapps/common/Geometry Dash layout:\n{game_path}"),
            steps: "No action needed.".into(),
        });
    } else {
        findings.push(DiagnosticFinding {
            severity: DiagnosticSeverity::Warning,
            title: "Install folder is outside a Steam library".into(),
            detail: format!("Path '{game_path}' is an isolated/custom slot or non-Steam copy."),
            steps: "If using isolated Version Slots, this is expected. Otherwise switch to your Steam install.".into(),
        });
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(prob) = validate_windows_game_images(root) {
            findings.push(DiagnosticFinding {
                severity: DiagnosticSeverity::Error,
                title: "Windows executable/DLL validation issue".into(),
                detail: prob,
                steps: "Close Geometry Dash, then use Steam -> Properties -> Installed Files -> Verify integrity.".into(),
            });
        } else {
            findings.push(DiagnosticFinding {
                severity: DiagnosticSeverity::Ok,
                title: "PE binaries & DLL architectures match".into(),
                detail: "GeometryDash.exe and all DLLs passed PE header and architecture checks.".into(),
                steps: "No action needed.".into(),
            });
        }

        let (vc_ok, vc_status) = check_vc_redist_installed();
        findings.push(DiagnosticFinding {
            severity: if vc_ok {
                DiagnosticSeverity::Ok
            } else {
                DiagnosticSeverity::Error
            },
            title: format!("Visual C++ x86 Redistributable: {vc_status}"),
            detail: if vc_ok {
                "Required C++ runtime libraries for Geometry Dash and Geode are present.".into()
            } else {
                "Geode requires Microsoft Visual C++ 2015-2022 Redistributable (x86) to avoid MSVCP140/VCRUNTIME140 crashes.".into()
            },
            steps: if vc_ok {
                "No action needed.".into()
            } else {
                "Download and install the official Microsoft VC++ 2015-2022 x86 redistributable.".into()
            },
        });
    }

    #[cfg(target_os = "macos")]
    {
        match find_macos_game_app(root) {
            Some(app) => match validate_macos_game_app(&app) {
                Ok(()) => findings.push(DiagnosticFinding {
                    severity: DiagnosticSeverity::Ok,
                    title: "macOS Geometry Dash.app bundle verified".into(),
                    detail: format!("Validated Info.plist and executable bit in {}", app.display()),
                    steps: "No action needed.".into(),
                }),
                Err(e) => findings.push(DiagnosticFinding {
                    severity: DiagnosticSeverity::Error,
                    title: "Geometry Dash.app looks incomplete".into(),
                    detail: e,
                    steps: "Verify integrity of game files in Steam.".into(),
                }),
            },
            None => findings.push(DiagnosticFinding {
                severity: DiagnosticSeverity::Error,
                title: "Geometry Dash.app not found".into(),
                detail: "Expected Geometry Dash.app inside the selected folder.".into(),
                steps: "Select the Steam folder containing Geometry Dash.app.".into(),
            }),
        }
    }

    #[cfg(target_os = "linux")]
    {
        if root.join("GeometryDash.exe").is_file() {
            findings.push(DiagnosticFinding {
                severity: DiagnosticSeverity::Ok,
                title: "Linux/Proton GeometryDash.exe confirmed".into(),
                detail: "GeoDeck manages the Windows GeometryDash.exe launched by Steam Proton.".into(),
                steps: "No action needed.".into(),
            });
        } else {
            findings.push(DiagnosticFinding {
                severity: DiagnosticSeverity::Error,
                title: "Proton GeometryDash.exe is missing".into(),
                detail: "The selected folder does not contain GeometryDash.exe.".into(),
                steps: "Verify integrity of game files in Steam.".into(),
            });
        }
    }

    let (writable, write_msg, free_mb) = check_folder_writable_and_space(root);
    findings.push(DiagnosticFinding {
        severity: if writable {
            DiagnosticSeverity::Ok
        } else {
            DiagnosticSeverity::Error
        },
        title: if writable {
            format!("Game folder writable ({free_mb} MB free)")
        } else {
            "Game folder write / disk space problem".into()
        },
        detail: write_msg,
        steps: if writable {
            "No action needed.".into()
        } else {
            "Free up disk/OneDrive space or check folder write permissions.".into()
        },
    });

    findings.extend(scan_texture_resources(game_path));

    if game_path.to_lowercase().contains("onedrive") {
        findings.push(DiagnosticFinding {
            severity: DiagnosticSeverity::Warning,
            title: "Geometry Dash is located inside Microsoft OneDrive".into(),
            detail: "OneDrive cloud placeholders can cause 0x8007016A errors or lock game/mod files.".into(),
            steps: "Click 'Pin OneDrive Local' or move your Steam library out of OneDrive.".into(),
        });
    }
    if save_dir.to_string_lossy().to_lowercase().contains("onedrive") {
        findings.push(DiagnosticFinding {
            severity: DiagnosticSeverity::Warning,
            title: "Geometry Dash save folder is synced by OneDrive".into(),
            detail: format!("Save path '{}' is inside OneDrive.", save_dir.display()),
            steps: "Right-click the save folder in Explorer and enable 'Always keep on this device'.".into(),
        });
    }

    let all_installs = find_all_gd_installations();
    if all_installs.len() > 1 {
        let lines: Vec<String> = all_installs
            .iter()
            .map(|inst| format!("• [{}] {} — GD {}", inst.source, inst.path, inst.version))
            .collect();
        findings.push(DiagnosticFinding {
            severity: DiagnosticSeverity::Warning,
            title: format!("Multiple Geometry Dash installations detected ({})", all_installs.len()),
            detail: lines.join("\n"),
            steps: "Ensure the install you actually launch in Steam is marked Active in GeoDeck -> Installs.".into(),
        });
    }

    findings
}

/// Maps common Geometry Dash / Geode / Steam / OS error messages to plain-English recovery steps.
pub fn explain_error_text(error_text: &str) -> (String, String) {
    let raw = error_text.trim();
    if raw.is_empty() {
        return (
            "Paste the exact error message".into(),
            "Copy the full message from the error popup (including codes like 0xc000007b or WinError 32) and click Explain Error.".into(),
        );
    }
    let text = raw.to_lowercase();

    if text.contains("whoopsies") && text.contains("unhandled exception") {
        return (
            "Geode Crashed (Unhandled Exception)".into(),
            "\"Whoopsies! An unhandled exception has occurred\" is Geode's generic crash screen.\n\nOpen Tools -> Crash Logs in GeoDeck to see the exact Faulty Mod ID and disable or downgrade it cleanly without deleting saves or DLLs.".into(),
        );
    }
    if (text.contains("damaged") && (text.contains("can't be opened") || text.contains("can’t be opened")))
        || text.contains("move it to the trash")
    {
        return (
            "macOS App Damaged Alert".into(),
            "This alert indicates an incomplete download or broken bundle signature. Confirm you downloaded GeoDeck from the official GitHub Releases page and extract the full .zip instead of overriding a damaged app warning.".into(),
        );
    }
    if text.contains("cannot be opened because apple cannot check")
        || text.contains("unidentified developer")
        || text.contains("developer cannot be verified")
    {
        return (
            "macOS Gatekeeper Approval Needed".into(),
            "For an unsigned/ad-hoc build downloaded from GitHub Releases: try opening GeoDeck once, then go to System Settings -> Privacy & Security -> Open Anyway if you verified the release checksum.".into(),
        );
    }
    if text.contains("0xc000007b") || text.contains("bad image") {
        return (
            "Windows 0xc000007b / Bad Image Architecture Mismatch".into(),
            "1. Close Geometry Dash and Steam.\n2. Run Tools -> Health Check to spot mismatched x86/x64 DLLs.\n3. In Steam -> Library -> Geometry Dash -> Properties -> Installed Files -> Verify integrity of game files.\n4. Never download random DLLs from the web.".into(),
        );
    }
    if text.contains("winerror 32")
        || text.contains("winerror 33")
        || text.contains("used by another process")
        || text.contains("being used by")
    {
        return (
            "File Locked by Running Game or Steam".into(),
            "GeometryDash.exe or Steam is holding a lock on files. Close Geometry Dash cleanly (or click Stop Game on Home), wait 2 seconds, and retry.".into(),
        );
    }
    if text.contains("prevter.imageplus") || text.contains("entry point not found") || text.contains("loadrootsubdictfromcompressedfile") {
        return (
            "Incompatible Mod (DLL Entry Point Crash)".into(),
            "A mod compiled for another GD version (such as prevter.imageplus v1.1.1 on GD 2.2081) is crashing startup.\n\nOpen Mods -> Audit & Keybinds and disable or downgrade the flagged mod.".into(),
        );
    }
    if text.contains("visual c++") || text.contains("vc++") || text.contains("vcredist") || text.contains("msvcp140") || text.contains("vcruntime140") {
        return (
            "Microsoft Visual C++ x86 Redistributable Missing or Outdated".into(),
            "Geode and Geometry Dash require Microsoft Visual C++ 2015-2022 Redistributable (x86). Install the official package from Microsoft Learn / aka.ms/vs/17/release/vc_redist.x86.exe.".into(),
        );
    }
    if text.contains("onedrive") || text.contains("0x8007016a") || text.contains("cloud file provider") || text.contains("winerror 392") {
        return (
            "OneDrive Cloud-Only Placeholder Conflict".into(),
            "Game or save files are stored inside OneDrive and marked online-only.\n\nUse Tools -> Health Check -> Pin OneDrive Local, or right-click the folder in Explorer -> 'Always keep on this device'.".into(),
        );
    }
    if text.contains("appmanifest_322170") || text.contains("manifest locked") {
        return (
            "Steam Update Manifest Is Write-Protected".into(),
            "appmanifest_322170.acf is locked (read-only) to stop Steam from auto-updating over an older GD build. Toggle Steam Update Lock off on Home or Installs if you want Steam to update.".into(),
        );
    }
    if text.contains("ccgamemanager") || text.contains("save file corrupted") || text.contains("cclocallevels") {
        return (
            "Geometry Dash Save Data Issue".into(),
            "Your CCGameManager.dat or CCLocalLevels.dat failed XML validation or was missing.\n\nOpen Backups -> Save Slots & Salvage to recover the latest valid backup or .dat.bak file automatically.".into(),
        );
    }
    if text.contains("storage full") || text.contains("quota") || text.contains("winerror 112") || text.contains("not enough space") {
        return (
            "Disk or Cloud Quota Full".into(),
            "The drive or OneDrive account is out of storage space. Free at least 500 MB and re-run Tools -> Health Check.".into(),
        );
    }
    if text.contains("2.207") {
        return (
            "Older Geometry Dash 2.207x Build".into(),
            "This slot runs GD 2.207x (Geode v4 era). Either update to 2.2081/2.2091 or match mod versions using Mods -> Audit & Keybinds.".into(),
        );
    }
    if text.contains("disk write error") {
        return (
            "Steam Disk Write Error".into(),
            "Close Geometry Dash, check free space, unlock Steam Update Lock in Installs if enabled, and run Steam -> Verify integrity of game files.".into(),
        );
    }
    if text.contains("access is denied") || text.contains("permission denied") || text.contains("winerror 5") {
        return (
            "File Access / Permission Denied".into(),
            "Ensure Geometry Dash and Steam are closed, unlock Steam Update Lock if enabled, and verify your antivirus isn't blocking writes to the game directory.".into(),
        );
    }
    if text.contains("steam") && (text.contains("not running") || text.contains("not found")) {
        return (
            "Steam Not Detected or Not Running".into(),
            "Open Steam and sign in first, then click Refresh or select your Steam Geometry Dash folder in Installs.".into(),
        );
    }

    (
        "Unrecognized Error Message".into(),
        "GeoDeck did not match a built-in signature for this text. Use Export Support Report in Tools -> Health Check and share both the report and your exact error text for help.".into(),
    )
}

/// Generates a privacy-redacted plain-text diagnostic report suitable for sharing in support channels.
pub fn generate_support_report(
    cfg: &Config,
    probe: &InstallationProbe,
    integrity: &IntegrityReport,
    mods: &[InstalledMod],
    findings: &[DiagnosticFinding],
) -> String {
    let ver = crate::core::updater::current_app_display_version();
    let locked = if is_manifest_locked(&cfg.game_path) {
        "Locked (Read-Only)"
    } else {
        "Unlocked"
    };
    let enabled_cnt = mods.iter().filter(|m| m.enabled).count();

    let mut out = format!(
        "=== GeoDeck {ver} Support Report ===\nOS: {} ({})\nActive Slot: {}\nGame Path: {}\nDetected GD Version: {}\nGeode Version: {}\nSteam Manifest Lock: {locked}\nCore Integrity: {} ({} verified, {} missing, {} corrupted)\nInstalled Mods: {} total ({enabled_cnt} enabled)\n\n== Health Check Findings ==\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
        cfg.active_version,
        cfg.game_path,
        probe.gd_version.as_deref().unwrap_or("Unknown"),
        probe.geode_version.as_deref().unwrap_or("Not Installed"),
        if integrity.overall_ok { "PASS" } else { "ISSUES FOUND" },
        integrity.healthy_count,
        integrity.missing_count,
        integrity.corrupted_count,
        mods.len(),
    );

    for f in findings {
        out.push_str(&format!(
            "- [{:?}] {}: {}\n",
            f.severity,
            f.title,
            f.detail.replace('\n', " ")
        ));
    }

    out.push_str("\n== Mods ==\n");
    for m in mods {
        out.push_str(&format!(
            "- [{}] {} {} (v{}, targets GD {})\n",
            if m.enabled { "ON " } else { "OFF" },
            m.id,
            m.name,
            m.version,
            m.gd_win_target.as_deref().unwrap_or("*"),
        ));
    }

    if cfg.redact_crash_usernames {
        if let Some(user) = current_username() {
            out = out
                .replace(&format!("Users\\{user}"), "Users\\<REDACTED>")
                .replace(&format!("Users/{user}"), "Users/<REDACTED>")
                .replace(&format!("/home/{user}"), "/home/<REDACTED>");
        }
    }
    out
}
