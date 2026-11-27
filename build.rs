use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn find_rc_exe() -> Option<PathBuf> {
    if let Ok(output) = Command::new("where").arg("rc.exe").output() {
        if output.status.success() {
            if let Some(first) = String::from_utf8_lossy(&output.stdout).lines().next() {
                let p = PathBuf::from(first.trim());
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }

    let kits_bin = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    if let Ok(rd) = fs::read_dir(kits_bin) {
        let mut candidates: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path().join("x64").join("rc.exe"))
            .filter(|p| p.is_file())
            .collect();
        candidates.sort();
        if let Some(newest) = candidates.pop() {
            return Some(newest);
        }
    }
    None
}

fn main() {
    println!("cargo:rerun-if-changed=assets/geodeck.ico");
    println!("cargo:rerun-if-changed=build.rs");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let ico_path = manifest_dir.join("assets").join("geodeck.ico");
    if !ico_path.is_file() {
        println!("cargo:warning=assets/geodeck.ico not found; skipping Windows icon embedding");
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let rc_path = out_dir.join("geodeck.rc");
    let res_path = out_dir.join("geodeck.res");

    let escaped_ico = ico_path.to_string_lossy().replace('\\', "\\\\");
    let rc_content = format!("1 ICON \"{escaped_ico}\"\n");

    if let Err(e) = fs::write(&rc_path, rc_content) {
        println!("cargo:warning=Failed to write geodeck.rc: {e}");
        return;
    }

    let Some(rc_exe) = find_rc_exe() else {
        println!("cargo:warning=Windows SDK rc.exe not found; skipping .ico resource compile");
        return;
    };

    match Command::new(&rc_exe)
        .args(["/nologo", "/fo"])
        .arg(&res_path)
        .arg(&rc_path)
        .output()
    {
        Ok(out) if out.status.success() && res_path.is_file() => {
            println!("cargo:rustc-link-arg-bins={}", res_path.display());
        }
        Ok(out) => {
            println!(
                "cargo:warning=rc.exe failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Err(e) => {
            println!("cargo:warning=Failed to invoke rc.exe: {e}");
        }
    }
}
