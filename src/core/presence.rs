use serde_json::{json, Value};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::core::updater::current_app_display_version;

const DISCORD_CLIENT_ID: &str = "1563607447384367195";
const GITHUB_URL: &str = "https://github.com/willmakesrandomshit/GeoDeck";

#[derive(Clone, Debug)]
struct PresencePayload {
    details: String,
    state: String,
    start_epoch: u64,
    dirty: bool,
}

struct SharedState {
    enabled: bool,
    connected: bool,
    stop: bool,
    payload: PresencePayload,
}

/// Cross-platform native Discord Rich Presence client speaking Discord's binary IPC protocol
/// over `\\.\pipe\discord-ipc-0..9` (Windows) or `discord-ipc-0..9` Unix domain sockets.
#[derive(Clone)]
pub struct DiscordRpcHandle {
    inner: Arc<(Mutex<SharedState>, Condvar)>,
    alive_flag: Arc<AtomicBool>,
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl DiscordRpcHandle {
    pub fn new(enabled: bool) -> Self {
        let now = unix_now_secs();
        let inner = Arc::new((
            Mutex::new(SharedState {
                enabled,
                connected: false,
                stop: false,
                payload: PresencePayload {
                    details: "GeoDeck · Geometry Dash Companion".into(),
                    state: "Vault & Mods Protected".into(),
                    start_epoch: now,
                    dirty: true,
                },
            }),
            Condvar::new(),
        ));
        let alive_flag = Arc::new(AtomicBool::new(true));

        let loop_inner = Arc::clone(&inner);
        let loop_alive = Arc::clone(&alive_flag);
        thread::Builder::new()
            .name("geodeck-discord-rpc".into())
            .spawn(move || rpc_worker_loop(loop_inner, loop_alive))
            .ok();

        Self { inner, alive_flag }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let (lock, cv) = &*self.inner;
        if let Ok(mut st) = lock.lock() {
            if st.enabled != enabled {
                st.enabled = enabled;
                if enabled {
                    st.payload.start_epoch = unix_now_secs();
                }
                st.payload.dirty = true;
                cv.notify_all();
            }
        }
    }

    pub fn update(&self, details: impl Into<String>, state: impl Into<String>, reset_timer: bool) {
        let details = details.into();
        let state = state.into();
        let (lock, cv) = &*self.inner;
        if let Ok(mut st) = lock.lock() {
            let mut changed = false;
            if st.payload.details != details {
                st.payload.details = details;
                changed = true;
            }
            if st.payload.state != state {
                st.payload.state = state;
                changed = true;
            }
            if reset_timer {
                st.payload.start_epoch = unix_now_secs();
                changed = true;
            }
            if changed {
                st.payload.dirty = true;
                cv.notify_all();
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        let (lock, _) = &*self.inner;
        lock.lock()
            .map(|st| st.enabled && st.connected)
            .unwrap_or(false)
    }

    pub fn shutdown(&self) {
        self.alive_flag.store(false, Ordering::Relaxed);
        let (lock, cv) = &*self.inner;
        if let Ok(mut st) = lock.lock() {
            st.stop = true;
            cv.notify_all();
        }
    }
}

trait IpcTransport: Read + Write + Send {}
impl<T: Read + Write + Send> IpcTransport for T {}

fn encode_frame(opcode: u32, json_val: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(json_val).unwrap_or_default();
    let mut pkt = Vec::with_capacity(8 + body.len());
    pkt.extend_from_slice(&opcode.to_le_bytes());
    pkt.extend_from_slice(&(body.len() as u32).to_le_bytes());
    pkt.extend_from_slice(&body);
    pkt
}

fn read_frame(stream: &mut dyn IpcTransport) -> Result<(u32, Vec<u8>), ()> {
    let mut hdr = [0u8; 8];
    stream.read_exact(&mut hdr).map_err(|_| ())?;
    let op = u32::from_le_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
    let len = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
    if len > 1024 * 1024 {
        return Err(());
    }
    let mut buf = vec![0u8; len];
    if len > 0 {
        stream.read_exact(&mut buf).map_err(|_| ())?;
    }
    Ok((op, buf))
}

fn connect_discord_ipc() -> Option<Box<dyn IpcTransport>> {
    #[cfg(target_os = "windows")]
    {
        for idx in 0..10 {
            let pipe = format!(r"\\.\pipe\discord-ipc-{idx}");
            if let Ok(mut file) = std::fs::OpenOptions::new().read(true).write(true).open(&pipe) {
                let hs = encode_frame(0, &json!({ "v": 1, "client_id": DISCORD_CLIENT_ID }));
                if file.write_all(&hs).is_ok() {
                    if let Ok((op, _)) = read_frame(&mut file) {
                        if op == 1 {
                            return Some(Box::new(file));
                        }
                    }
                }
            }
        }
        None
    }

    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        let mut bases = Vec::new();
        for env_key in ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"] {
            if let Some(val) = std::env::var_os(env_key) {
                let p = std::path::PathBuf::from(val);
                bases.push(p.clone());
                bases.push(p.join("app").join("com.discordapp.Discord"));
            }
        }
        bases.push(std::path::PathBuf::from("/tmp"));

        for base in bases {
            for idx in 0..10 {
                let sock_path = base.join(format!("discord-ipc-{idx}"));
                if sock_path.exists() {
                    if let Ok(mut stream) = UnixStream::connect(&sock_path) {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
                        let hs =
                            encode_frame(0, &json!({ "v": 1, "client_id": DISCORD_CLIENT_ID }));
                        if stream.write_all(&hs).is_ok() {
                            if let Ok((op, _)) = read_frame(&mut stream) {
                                if op == 1 {
                                    return Some(Box::new(stream));
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    #[cfg(not(any(target_os = "windows", unix)))]
    {
        None
    }
}

fn send_activity(stream: &mut dyn IpcTransport, payload: Option<&PresencePayload>) -> bool {
    let pid = std::process::id();
    let nonce = format!("gd-{}", unix_now_secs());
    let ver = current_app_display_version();

    let activity = match payload {
        Some(p) => json!({
            "details": p.details,
            "state": p.state,
            "timestamps": { "start": p.start_epoch },
            "assets": {
                "large_image": format!("https://raw.githubusercontent.com/willmakesrandomshit/GeoDeck/main/assets/geodeck_icon.png"),
                "large_text": format!("GeoDeck {ver} · Geometry Dash Launcher"),
                "small_image": format!("https://raw.githubusercontent.com/willmakesrandomshit/GeoDeck/main/assets/geodeck_icon.png"),
                "small_text": "GeoDeck"
            },
            "buttons": [
                { "label": "View GeoDeck on GitHub", "url": GITHUB_URL }
            ],
            "metadata": {
                "button_urls": [GITHUB_URL]
            }
        }),
        None => Value::Null,
    };

    let cmd = json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": pid,
            "activity": activity
        },
        "nonce": nonce
    });
    let pkt = encode_frame(1, &cmd);
    if stream.write_all(&pkt).is_err() {
        return false;
    }
    read_frame(stream).is_ok()
}

fn rpc_worker_loop(inner: Arc<(Mutex<SharedState>, Condvar)>, alive: Arc<AtomicBool>) {
    let mut conn: Option<Box<dyn IpcTransport>> = None;
    let (lock, cv) = &*inner;

    while alive.load(Ordering::Relaxed) {
        let (enabled, stop, payload_snapshot, dirty) = {
            let Ok(mut st) = lock.lock() else {
                break;
            };
            let d = st.payload.dirty;
            st.payload.dirty = false;
            (st.enabled, st.stop, st.payload.clone(), d)
        };

        if stop {
            if let Some(mut s) = conn.take() {
                let _ = send_activity(&mut *s, None);
            }
            if let Ok(mut st) = lock.lock() {
                st.connected = false;
            }
            break;
        }

        if !enabled {
            if let Some(mut s) = conn.take() {
                let _ = send_activity(&mut *s, None);
            }
            if let Ok(mut st) = lock.lock() {
                st.connected = false;
            }
        } else {
            let need_connect = conn.is_none();
            if need_connect {
                conn = connect_discord_ipc();
            }
            if let Some(ref mut s) = conn {
                if need_connect || dirty {
                    if !send_activity(&mut **s, Some(&payload_snapshot)) {
                        conn = None;
                    }
                }
            }
            if let Ok(mut st) = lock.lock() {
                st.connected = conn.is_some();
            }
        }

        let wait_dur = if conn.is_some() {
            Duration::from_secs(10)
        } else {
            Duration::from_secs(8)
        };
        if let Ok(st) = lock.lock() {
            if !st.stop && !st.payload.dirty {
                let _ = cv.wait_timeout(st, wait_dur);
            }
        }
    }
}
