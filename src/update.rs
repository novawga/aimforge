//! Self-update from GitHub Releases, plus downloading the community playlists.
//!
//! How releases work: push a tag like `v0.3.0`; the GitHub Actions workflow
//! builds every platform and attaches the binaries to a release. On launch
//! AimForge asks GitHub for the latest release, and if it is newer it downloads
//! the binary for this platform, checks its SHA-256, swaps it in place of the
//! running executable and restarts.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

/// GitHub repository that publishes releases.
pub const REPO: &str = "novawga/aimforge";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Release asset name for this platform.
pub fn asset_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "aimforge-windows-x86_64.exe"
    } else if cfg!(target_os = "macos") {
        "aimforge-macos-arm64"
    } else {
        "aimforge-linux-x86_64"
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Idle,
    Checking,
    UpToDate,
    Downloading { version: String, done: u64, total: u64 },
    /// New binary is in place; restart to run it.
    Installed { version: String },
    /// Newer version exists but this copy can't replace itself (dev build, read-only folder...).
    Available { version: String, url: String, why: String },
    Failed(String),
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
    /// "sha256:<hex>", provided by GitHub for release assets.
    #[serde(default)]
    digest: Option<String>,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(60))
        .try_proxy_from_env(true)
        .user_agent(&format!("AimForge/{VERSION}"))
        .build()
}

/// "v1.2.3" -> [1, 2, 3]
pub fn parse_version(s: &str) -> Vec<u64> {
    s.trim()
        .trim_start_matches(['v', 'V'])
        .split(['.', '-', '+'])
        .take(3)
        .map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0))
        .collect()
}

pub fn is_newer(remote: &str, local: &str) -> bool {
    let (mut a, mut b) = (parse_version(remote), parse_version(local));
    a.resize(3, 0);
    b.resize(3, 0);
    a > b
}

/// Why this copy shouldn't overwrite itself, if any.
fn self_update_blocker(exe: &Path) -> Option<String> {
    if std::env::var_os("AIMFORGE_NO_UPDATE").is_some() {
        return Some("updates disabled by AIMFORGE_NO_UPDATE".into());
    }
    if cfg!(debug_assertions) {
        return Some("debug build".into());
    }
    let s = exe.to_string_lossy().replace('\\', "/");
    if s.contains("/target/release/") || s.contains("/target/debug/") {
        return Some("running from a cargo build folder (use `git pull` + `cargo run --release`)".into());
    }
    None
}

pub struct Updater {
    rx: Receiver<Status>,
    tx: Sender<Status>,
    pub status: Status,
    pub notes_url: Option<String>,
    /// Executable path captured at startup. After the swap the OS may report the
    /// renamed old file as "current exe", so we never ask again.
    exe: Option<PathBuf>,
    /// Version we just updated to, when this process was launched by an update.
    pub just_updated: Option<String>,
}

const UPDATED_ENV: &str = "AIMFORGE_UPDATED_TO";

impl Updater {
    pub fn new() -> Updater {
        let (tx, rx) = channel();
        let exe = std::env::current_exe().ok().map(|e| e.canonicalize().unwrap_or(e));
        // Clean up the previous executable left behind by the last update.
        if let Some(exe) = &exe {
            let _ = std::fs::remove_file(old_path(exe));
        }
        let just_updated = std::env::var(UPDATED_ENV).ok();
        if just_updated.is_some() {
            std::env::remove_var(UPDATED_ENV);
        }
        Updater { rx, tx, status: Status::Idle, notes_url: None, exe, just_updated }
    }

    /// Start a background check (and install if `install` is true).
    pub fn check(&mut self, install: bool) {
        if matches!(self.status, Status::Checking | Status::Downloading { .. }) {
            return;
        }
        let Some(exe) = self.exe.clone() else {
            self.status = Status::Failed("can't locate the AimForge executable".into());
            return;
        };
        self.status = Status::Checking;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = run(install, &exe, &tx);
            let _ = tx.send(match result {
                Ok(s) => s,
                Err(e) => Status::Failed(e),
            });
        });
    }

    /// Pull progress from the worker thread. Returns true if the status changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(s) = self.rx.try_recv() {
            if let Status::Available { url, .. } = &s {
                self.notes_url = Some(url.clone());
            }
            self.status = s;
            changed = true;
        }
        changed
    }

    /// Launch the (new) executable at the original path and exit this one.
    pub fn restart(&self) -> ! {
        if let Some(exe) = &self.exe {
            let args: Vec<String> = std::env::args().skip(1).collect();
            let mut cmd = std::process::Command::new(exe);
            cmd.args(args);
            if let Status::Installed { version } = &self.status {
                // Loop guard: the new process won't try to update again this launch.
                cmd.env(UPDATED_ENV, version);
            }
            if let Some(dir) = exe.parent() {
                cmd.current_dir(dir);
            }
            let _ = cmd.spawn();
        }
        std::process::exit(0);
    }
}

fn old_path(exe: &Path) -> PathBuf {
    let mut s = exe.as_os_str().to_owned();
    s.push(".old");
    PathBuf::from(s)
}

/// API host; overridable for testing against a local mock.
fn api_base() -> String {
    std::env::var("AIMFORGE_UPDATE_API").unwrap_or_else(|_| "https://api.github.com".into())
}

fn run(install: bool, exe: &Path, tx: &Sender<Status>) -> Result<Status, String> {
    let url = format!("{}/repos/{REPO}/releases/latest", api_base());
    let resp = agent()
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => "no releases published yet".to_string(),
            other => format!("couldn't reach GitHub: {other}"),
        })?;
    let body = resp.into_string().map_err(|e| format!("bad response from GitHub: {e}"))?;
    let rel: Release = serde_json::from_str(&body).map_err(|e| format!("bad response from GitHub: {e}"))?;
    if rel.draft || rel.prerelease || !is_newer(&rel.tag_name, VERSION) {
        return Ok(Status::UpToDate);
    }
    let version = rel.tag_name.trim_start_matches(['v', 'V']).to_string();
    let Some(asset) = rel.assets.iter().find(|a| a.name == asset_name()) else {
        return Ok(Status::Available { version, url: rel.html_url, why: format!("no {} in this release yet", asset_name()) });
    };
    let exe = exe.to_path_buf();
    if let Some(why) = self_update_blocker(&exe) {
        return Ok(Status::Available { version, url: rel.html_url, why });
    }
    if !install {
        return Ok(Status::Available { version, url: rel.html_url, why: String::new() });
    }

    // Download.
    let resp = agent().get(&asset.browser_download_url).call().map_err(|e| format!("download failed: {e}"))?;
    let total = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(asset.size);
    let mut reader = resp.into_reader();
    let mut data = Vec::with_capacity(total as usize);
    let mut buf = [0u8; 64 * 1024];
    let mut last_report = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("download failed: {e}"))?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        let done = data.len() as u64;
        if done - last_report > 256 * 1024 {
            last_report = done;
            let _ = tx.send(Status::Downloading { version: version.clone(), done, total });
        }
    }
    if asset.size > 0 && data.len() as u64 != asset.size {
        return Err(format!("download incomplete ({} of {} bytes)", data.len(), asset.size));
    }
    if let Some(d) = &asset.digest {
        if let Some(want) = d.strip_prefix("sha256:") {
            let got = hex(&Sha256::digest(&data));
            if !got.eq_ignore_ascii_case(want) {
                return Err("downloaded file failed its checksum; update skipped".into());
            }
        }
    }

    // Swap the executable: running files can be renamed (even on Windows), so
    // move the current one aside and put the new one in its place.
    let new_path = {
        let mut s = exe.as_os_str().to_owned();
        s.push(".new");
        PathBuf::from(s)
    };
    std::fs::write(&new_path, &data).map_err(|e| format!("couldn't write update next to the app: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&new_path, std::fs::Permissions::from_mode(0o755));
    }
    let old = old_path(&exe);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| {
        let _ = std::fs::remove_file(&new_path);
        format!("couldn't replace the app: {e}")
    })?;
    if let Err(e) = std::fs::rename(&new_path, &exe) {
        let _ = std::fs::rename(&old, &exe); // roll back
        return Err(format!("couldn't install update: {e}"));
    }
    Ok(Status::Installed { version })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Community playlists
// ---------------------------------------------------------------------------

pub const PLAYLIST_REPO: &str = "riddbtw/kovaaks-playlists";

/// Download the community playlist repo and unpack its .json/.plo files into
/// `dest` (replacing what was there). Returns the number of files written.
pub fn download_playlists(dest: &Path) -> Result<usize, String> {
    let url = format!("https://codeload.github.com/{PLAYLIST_REPO}/zip/refs/heads/main");
    let resp = agent().get(&url).call().map_err(|e| format!("couldn't download playlists: {e}"))?;
    let mut data = vec![];
    resp.into_reader()
        .take(64 * 1024 * 1024)
        .read_to_end(&mut data)
        .map_err(|e| format!("couldn't download playlists: {e}"))?;
    extract_playlists(&data, dest)
}

pub fn extract_playlists(zip_bytes: &[u8], dest: &Path) -> Result<usize, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).map_err(|e| format!("bad archive: {e}"))?;
    let tmp = dest.with_extension("downloading");
    let _ = std::fs::remove_dir_all(&tmp);
    let mut count = 0;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        let Some(path) = f.enclosed_name() else { continue };
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if f.is_dir() || !(ext == "json" || ext == "plo") || f.size() > 4 * 1024 * 1024 {
            continue;
        }
        // Drop the top-level "<repo>-main/" folder.
        let rel: PathBuf = path.components().skip(1).collect();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out = tmp.join(&rel);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut bytes = vec![];
        f.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        std::fs::write(&out, bytes).map_err(|e| e.to_string())?;
        count += 1;
    }
    if count == 0 {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err("no playlists found in the download".into());
    }
    let _ = std::fs::remove_dir_all(dest);
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    let _ = std::fs::write(
        dest.join("SOURCE.txt"),
        format!("Downloaded from https://github.com/{PLAYLIST_REPO}\nCommunity playlists by their respective authors.\n"),
    );
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(is_newer("v0.3.0", "0.2.0"));
        assert!(is_newer("v1.0", "0.9.9"));
        assert!(!is_newer("v0.2.0", "0.2.0"));
        assert!(!is_newer("0.1.9", "0.2.0"));
        assert!(is_newer("v0.2.10", "0.2.9"));
    }

    #[test]
    fn extract_zip() {
        use std::io::Write;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            w.start_file("kovaaks-playlists-main/benches/a.json", opt).unwrap();
            w.write_all(br#"{"playlistName":"A","scenarioList":[]}"#).unwrap();
            w.start_file("kovaaks-playlists-main/README.md", opt).unwrap();
            w.write_all(b"hi").unwrap();
            w.start_file("kovaaks-playlists-main/x/b.plo", opt).unwrap();
            w.write_all(b"GVAS").unwrap();
            w.finish().unwrap();
        }
        let dir = std::env::temp_dir().join(format!("af_zip_test_{}", std::process::id()));
        let dest = dir.join("community");
        assert_eq!(extract_playlists(buf.get_ref(), &dest).unwrap(), 2);
        assert!(dest.join("benches/a.json").exists());
        assert!(dest.join("x/b.plo").exists());
        assert!(!dest.join("README.md").exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod live_tests {
    /// Network: `cargo test --no-default-features live_ -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn live_playlists() {
        let dest = std::env::temp_dir().join("af_live_pl");
        let n = super::download_playlists(&dest).unwrap();
        println!("downloaded {n} playlists to {}", dest.display());
        assert!(n > 100);
        let (pls, errs) = crate::playlist::load_dir(&dest);
        println!("parsed {} playlists, {} errors", pls.len(), errs.len());
        assert!(errs.is_empty());
    }

    #[test]
    #[ignore]
    fn live_release_api() {
        let repo = std::env::var("AF_TEST_REPO").unwrap_or("novawga/nova-clip".into());
        let body = super::agent()
            .get(&format!("https://api.github.com/repos/{repo}/releases/latest"))
            .call()
            .map(|r| r.into_string().unwrap());
        match body {
            Ok(b) => {
                let rel: super::Release = serde_json::from_str(&b).unwrap();
                println!("{repo}: latest {} with {} assets; digests: {:?}", rel.tag_name, rel.assets.len(),
                    rel.assets.iter().map(|a| (a.name.clone(), a.digest.clone())).collect::<Vec<_>>());
            }
            Err(e) => println!("{repo}: {e}"),
        }
    }
}
