use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::config::config_dir;

pub const VAULT_MAGIC: &[u8; 4] = b"GDV1";

pub fn default_vault_key_path() -> PathBuf {
    config_dir().join("vault.key.dpapi")
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(windows)]
pub fn dpapi_protect(plain: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: plain.len() as u32,
            pbData: plain.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = CryptProtectData(
            &mut input,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        );
        if ok == 0 {
            return Err("Windows DPAPI CryptProtectData failed".into());
        }
        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as *mut _);
        Ok(slice)
    }
}

#[cfg(windows)]
pub fn dpapi_unprotect(protected: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: protected.len() as u32,
            pbData: protected.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = CryptUnprotectData(
            &mut input,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        );
        if ok == 0 {
            return Err("Windows DPAPI CryptUnprotectData failed".into());
        }
        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as *mut _);
        Ok(slice)
    }
}

#[cfg(not(windows))]
pub fn dpapi_protect(plain: &[u8]) -> Result<Vec<u8>, String> {
    Ok(plain.to_vec())
}

#[cfg(not(windows))]
pub fn dpapi_unprotect(protected: &[u8]) -> Result<Vec<u8>, String> {
    Ok(protected.to_vec())
}

pub fn load_or_create_vault_key(key_path: &Path) -> Result<[u8; 32], String> {
    if key_path.is_file() {
        let protected = fs::read(key_path).map_err(|e| format!("read vault key: {e}"))?;
        let raw = dpapi_unprotect(&protected)?;
        if raw.len() != 32 {
            return Err("Decrypted vault key has invalid length".into());
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&raw);
        return Ok(key);
    }

    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create vault key dir: {e}"))?;
    }
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    let protected = dpapi_protect(&key)?;
    fs::write(key_path, &protected).map_err(|e| format!("write vault key: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(key_path, fs::Permissions::from_mode(0o600));
    }
    Ok(key)
}

pub fn vault_key_protection_label() -> &'static str {
    if cfg!(windows) {
        "Windows DPAPI"
    } else {
        "Local Key (0600)"
    }
}

pub fn encrypt_vault_payload(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("cipher init: {e}"))?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("AES-GCM encrypt failed: {e}"))?;

    let mut out = Vec::with_capacity(4 + 12 + ciphertext.len());
    out.extend_from_slice(VAULT_MAGIC);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt_vault_payload(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>, String> {
    if blob.len() < 16 || &blob[0..4] != VAULT_MAGIC {
        return Err("Invalid encrypted vault blob header".into());
    }
    let nonce = Nonce::from_slice(&blob[4..16]);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("cipher init: {e}"))?;
    cipher
        .decrypt(nonce, &blob[16..])
        .map_err(|_| "AES-GCM authentication failed (tampered blob or wrong key)".to_string())
}

pub fn dpapi_key_ready() -> bool {
    default_vault_key_path().is_file()
}
