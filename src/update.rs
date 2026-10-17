//! Checks GitHub Releases for newer versions and installs them in place,
//! following obsidian-web-gateway's updater.
//!
//! Scheduled checks only record what is available; installing always needs an
//! explicit request. Archives are verified against the release's
//! `SHA256SUMS.txt`, the extracted binary must report the expected version, and
//! the previous executable is kept as `*.old` for rollback.

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration as StdDuration,
};

use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, TimeZone};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::error::{AppError, AppResult};

pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_API: &str = "https://api.github.com/repos/zhangchaosd/homepage";
const SUMS: &str = "SHA256SUMS.txt";
const BINARY: &str = "homepage";
const NOTES_LIMIT: usize = 20_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// `off`, `daily` or `weekly`.
    pub schedule: String,
    /// ISO weekday for weekly checks: 1 = Monday … 7 = Sunday.
    pub weekday: u32,
    /// Server-local time of day, `HH:MM`.
    pub time: String,
    /// `stable` ignores prereleases; `prerelease` includes them.
    pub channel: String,
}

impl Default for Settings {
    /// A daily check only records that a release exists, so it is on by default;
    /// installing still needs a click.
    fn default() -> Self {
        Self {
            schedule: "daily".into(),
            weekday: 1,
            time: "04:00".into(),
            channel: "stable".into(),
        }
    }
}

impl Settings {
    pub fn validate(&self) -> AppResult<()> {
        let invalid = |message: &str| Err(AppError::InvalidRequest(message.into()));
        if !["off", "daily", "weekly"].contains(&self.schedule.as_str()) {
            return invalid("schedule must be off, daily, or weekly");
        }
        if !(1..=7).contains(&self.weekday) {
            return invalid("weekday must be between 1 (Monday) and 7 (Sunday)");
        }
        if NaiveTime::parse_from_str(&self.time, "%H:%M").is_err() {
            return invalid("time must use HH:MM");
        }
        if !["stable", "prerelease"].contains(&self.channel.as_str()) {
            return invalid("channel must be stable or prerelease");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Release {
    pub version: String,
    pub url: String,
    pub notes: String,
    pub published_at: String,
    pub prerelease: bool,
    pub asset_url: Option<String>,
    pub sums_url: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct State {
    pub settings: Settings,
    pub checked_at: Option<i64>,
    pub latest: Option<Release>,
    pub error: Option<String>,
}

pub struct Installed {
    pub version: String,
    pub executable: PathBuf,
}

/// Platform suffix used by `.github/workflows/release.yml`.
pub fn platform() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("linux-x86_64"),
        ("linux", "aarch64") => Some("linux-aarch64"),
        ("macos", "aarch64") => Some("macos-aarch64"),
        ("macos", "x86_64") => Some("macos-x86_64"),
        _ => None,
    }
}

/// Release archives carry no version so `releases/latest/download/…` links stay stable.
fn asset_name() -> Option<String> {
    platform().map(|platform| format!("homepage-{platform}.tar.gz"))
}

/// Replacing and re-executing the running binary needs Unix semantics.
pub fn installable() -> bool {
    cfg!(unix) && platform().is_some()
}

/// Picks the newest release above `current` from a GitHub releases listing.
pub fn select(releases: &Value, channel: &str, current: &str) -> Option<Release> {
    let current = Version::parse(current).ok()?;
    releases
        .as_array()?
        .iter()
        .filter(|release| !release["draft"].as_bool().unwrap_or(false))
        .filter(|release| channel != "stable" || !release["prerelease"].as_bool().unwrap_or(false))
        .filter_map(|release| {
            let tag = release["tag_name"].as_str()?;
            let version = Version::parse(tag.trim_start_matches('v')).ok()?;
            (version > current).then_some((version, release))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(version, release)| {
            let version = version.to_string();
            let asset = |name: &str| {
                release["assets"].as_array().and_then(|assets| {
                    assets
                        .iter()
                        .find(|asset| asset["name"] == name)
                        .and_then(|asset| asset["browser_download_url"].as_str())
                        .map(String::from)
                })
            };
            Release {
                url: release["html_url"].as_str().unwrap_or_default().into(),
                notes: release["body"]
                    .as_str()
                    .unwrap_or_default()
                    .chars()
                    .take(NOTES_LIMIT)
                    .collect(),
                published_at: release["published_at"].as_str().unwrap_or_default().into(),
                prerelease: release["prerelease"].as_bool().unwrap_or(false),
                asset_url: asset_name().and_then(|name| asset(&name)),
                sums_url: asset(SUMS),
                version,
            }
        })
}

/// Finds `name`'s digest in a `sha256sum` listing.
pub fn expected_digest(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, file) = line.split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name).then(|| hash.to_ascii_lowercase())
    })
}

/// Whether a scheduled check is due: the latest scheduled slot at or before
/// `now` has not been covered by a check yet. A slot missed while the server
/// was down is caught up once.
pub fn due(settings: &Settings, last_check: Option<i64>, now: DateTime<Local>) -> bool {
    let Ok(time) = NaiveTime::parse_from_str(&settings.time, "%H:%M") else {
        return false;
    };
    let mut date = now.date_naive();
    let period = match settings.schedule.as_str() {
        "daily" => 1,
        "weekly" => {
            let today = now.weekday().number_from_monday();
            date -= Duration::days(((today + 7 - settings.weekday) % 7).into());
            7
        }
        _ => return false,
    };
    let slot_at =
        |day: chrono::NaiveDate| Local.from_local_datetime(&day.and_time(time)).earliest();
    let Some(mut slot) = slot_at(date) else {
        return false;
    };
    if slot > now {
        match slot_at(date - Duration::days(period)) {
            Some(previous) => slot = previous,
            None => return false,
        }
    }
    last_check.is_none_or(|checked| checked < slot.timestamp())
}

/// Extracts the homepage executable from a release `.tar.gz` archive.
pub fn extract_binary(archive: &[u8]) -> AppResult<Vec<u8>> {
    let mut entries = tar::Archive::new(flate2::read::GzDecoder::new(archive));
    for entry in entries.entries().map_err(invalid_archive)? {
        let mut entry = entry.map_err(invalid_archive)?;
        let is_binary = entry.header().entry_type().is_file()
            && entry
                .path()
                .map_err(invalid_archive)?
                .file_name()
                .is_some_and(|name| name == BINARY);
        if is_binary {
            let mut binary = Vec::new();
            entry.read_to_end(&mut binary).map_err(invalid_archive)?;
            return Ok(binary);
        }
    }
    Err(AppError::InvalidRequest(format!(
        "the release archive does not contain {BINARY}"
    )))
}

fn invalid_archive(error: std::io::Error) -> AppError {
    AppError::InvalidRequest(format!("invalid release archive: {error}"))
}

fn failed(context: &str, error: impl std::fmt::Display) -> AppError {
    AppError::InvalidRequest(format!("{context}: {error}"))
}

pub struct Updater {
    path: PathBuf,
    api: String,
    client: reqwest::Client,
    busy: Mutex<()>,
}

impl Updater {
    pub fn new(state_dir: &Path) -> AppResult<Self> {
        let client = reqwest::Client::builder()
            .user_agent(format!("homepage/{CURRENT}"))
            .connect_timeout(StdDuration::from_secs(10))
            .timeout(StdDuration::from_secs(180))
            .build()
            .map_err(|error| AppError::Internal(format!("http client: {error}")))?;
        Ok(Self {
            path: state_dir.join("update.json"),
            // Overridable so tests and forks can point at another releases API.
            api: std::env::var("HOMEPAGE_UPDATE_API").unwrap_or_else(|_| DEFAULT_API.into()),
            client,
            busy: Mutex::new(()),
        })
    }

    pub fn state(&self) -> State {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn save(&self, state: &State) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = self.path.with_extension("json.tmp");
        fs::write(
            &temporary,
            serde_json::to_vec_pretty(state)
                .map_err(|error| AppError::Internal(error.to_string()))?,
        )?;
        fs::rename(temporary, &self.path)?;
        Ok(())
    }

    fn try_busy(&self) -> AppResult<tokio::sync::MutexGuard<'_, ()>> {
        self.busy.try_lock().map_err(|_| {
            AppError::InvalidRequest("another update task is running; try again shortly".into())
        })
    }

    pub fn save_settings(&self, settings: Settings) -> AppResult<()> {
        settings.validate()?;
        let _guard = self.try_busy()?;
        let mut state = self.state();
        if state.settings.channel != settings.channel {
            // A result from the other channel would be misleading.
            state.latest = None;
        }
        state.settings = settings;
        self.save(&state)
    }

    pub fn status(&self) -> Value {
        let state = self.state();
        let current = Version::parse(CURRENT).ok();
        // A stored result becomes stale once this version has caught up with it.
        let latest = state.latest.filter(|release| {
            Version::parse(&release.version)
                .ok()
                .zip(current.clone())
                .is_some_and(|(latest, current)| latest > current)
        });
        json!({
            "current": CURRENT,
            "platform": platform(),
            "installable": installable(),
            "settings": state.settings,
            "checkedAt": state.checked_at,
            "error": state.error,
            "available": latest.is_some(),
            "latest": latest,
        })
    }

    pub fn last_checked(&self) -> Option<i64> {
        self.state().checked_at
    }

    async fn download(&self, url: &str, limit: usize) -> AppResult<Vec<u8>> {
        let context = format!("download failed for {url}");
        let mut response = self
            .client
            .get(url)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|error| failed(&context, error))?;
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| failed(&context, error))?
        {
            body.extend_from_slice(&chunk);
            if body.len() > limit {
                return Err(failed(&context, "response is too large"));
            }
        }
        Ok(body)
    }

    pub async fn check(&self) -> AppResult<Option<Release>> {
        let _guard = self.try_busy()?;
        let mut state = self.state();
        let result = async {
            let bytes = self
                .download(&format!("{}/releases?per_page=30", self.api), 8 << 20)
                .await
                .map_err(|error| failed("cannot reach GitHub", error))?;
            let releases: Value = serde_json::from_slice(&bytes)
                .map_err(|error| failed("unexpected GitHub response", error))?;
            Ok::<_, AppError>(select(&releases, &state.settings.channel, CURRENT))
        }
        .await;
        state.checked_at = Some(chrono::Utc::now().timestamp());
        match &result {
            Ok(latest) => {
                state.latest = latest.clone();
                state.error = None;
            }
            Err(error) => state.error = Some(error.to_string()),
        }
        self.save(&state)?;
        result
    }

    /// Runs a check when the schedule says one is due; errors are recorded in the state.
    pub async fn tick(&self) {
        let state = self.state();
        if due(&state.settings, state.checked_at, Local::now()) {
            match self.check().await {
                Ok(Some(release)) => {
                    tracing::info!(version = %release.version, "homepage update available")
                }
                Ok(None) => {}
                Err(error) => tracing::warn!(%error, "update check failed"),
            }
        }
    }

    /// Downloads, verifies, and swaps in `version`, which must be the last checked release.
    pub async fn install(&self, version: &str) -> AppResult<Installed> {
        if !installable() {
            return Err(AppError::InvalidRequest(
                "automatic installation is not supported on this platform; download the release manually".into(),
            ));
        }
        let _guard = self.try_busy()?;
        let release = self
            .state()
            .latest
            .filter(|release| release.version == version)
            .ok_or_else(|| {
                AppError::InvalidRequest(
                    "release information is stale; check for updates again".into(),
                )
            })?;
        let name = asset_name().unwrap_or_default();
        let asset_url = release
            .asset_url
            .as_deref()
            .ok_or_else(|| AppError::InvalidRequest(format!("this release has no {name}")))?;
        let sums_url = release.sums_url.as_deref().ok_or_else(|| {
            AppError::InvalidRequest(format!("this release has no {SUMS}; refusing to install"))
        })?;
        let sums = String::from_utf8(self.download(sums_url, 1 << 20).await?)
            .map_err(|error| failed("invalid checksum file", error))?;
        let expected = expected_digest(&sums, &name)
            .ok_or_else(|| AppError::InvalidRequest(format!("{SUMS} does not list {name}")))?;
        let archive = self.download(asset_url, 256 << 20).await?;
        if format!("{:x}", Sha256::digest(&archive)) != expected {
            return Err(AppError::InvalidRequest(format!(
                "checksum mismatch: the download does not match {SUMS}"
            )));
        }
        let version = version.to_owned();
        tokio::task::spawn_blocking(move || stage_and_swap(&archive, &version)).await?
    }
}

fn stage_and_swap(archive: &[u8], version: &str) -> AppResult<Installed> {
    let binary = extract_binary(archive)?;
    let executable = std::env::current_exe()?.canonicalize()?;
    let directory = executable
        .parent()
        .ok_or_else(|| AppError::Internal("cannot determine the program directory".into()))?;
    let staged = directory.join(format!(".homepage-{version}.download"));
    fs::write(&staged, &binary)
        .map_err(|error| failed("cannot write to the program directory", error))?;
    if let Err(error) = verify_staged(&staged, version) {
        let _ = fs::remove_file(&staged);
        return Err(error);
    }
    swap(&executable, &staged)?;
    Ok(Installed {
        version: version.into(),
        executable,
    })
}

fn verify_staged(staged: &Path, version: &str) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(staged, fs::Permissions::from_mode(0o755))?;
    }
    let output = std::process::Command::new(staged)
        .arg("--version")
        .output()?;
    let reported = String::from_utf8_lossy(&output.stdout);
    if output.status.success() && reported.trim() == format!("homepage {version}") {
        Ok(())
    } else {
        Err(AppError::InvalidRequest(format!(
            "the new version cannot run on this machine: {}",
            reported.trim()
        )))
    }
}

fn old_path(executable: &Path) -> PathBuf {
    let mut name = executable.file_name().unwrap_or_default().to_os_string();
    name.push(".old");
    executable.with_file_name(name)
}

/// Replaces `executable` with `staged`, keeping the previous binary as `*.old`.
fn swap(executable: &Path, staged: &Path) -> AppResult<()> {
    let old = old_path(executable);
    if old.exists() {
        fs::remove_file(&old)?;
    }
    fs::rename(executable, &old)?;
    if let Err(error) = fs::rename(staged, executable) {
        fs::rename(&old, executable)?;
        return Err(failed("cannot replace the executable", error));
    }
    Ok(())
}

/// Swaps the current binary with the `*.old` one kept by the last install.
pub fn rollback() -> AppResult<PathBuf> {
    let executable = std::env::current_exe()?.canonicalize()?;
    let old = old_path(&executable);
    if !old.exists() {
        return Err(AppError::InvalidRequest(format!(
            "no previous version to roll back to: {}",
            old.display()
        )));
    }
    let parked = executable.with_extension("rollback");
    fs::rename(&executable, &parked)?;
    fs::rename(&old, &executable)?;
    fs::rename(&parked, &old)?;
    Ok(executable)
}

/// Replaces this process with `executable`, keeping arguments, environment, and
/// PID so service managers see the same process. Rust opens descriptors with
/// close-on-exec, which releases the listener for the new binary.
#[cfg(unix)]
pub fn reexec(executable: &Path) -> ! {
    use std::os::unix::process::CommandExt;
    let error = std::process::Command::new(executable)
        .args(std::env::args_os().skip(1))
        .exec();
    eprintln!("restart after update failed: {error}");
    std::process::exit(1)
}

#[derive(Clone, Copy, Debug, clap::Subcommand)]
pub enum UpdateCommand {
    /// Print whether a newer release is available on the configured channel.
    Check,
    /// Download, verify against SHA256SUMS.txt, and replace this executable.
    Install {
        #[arg(long)]
        yes: bool,
    },
    /// Restore the executable kept as `*.old` by the last install.
    Rollback {
        #[arg(long)]
        yes: bool,
    },
}

/// `homepage update …` from the command line.
pub async fn run_command(command: UpdateCommand, state_dir: &Path) -> AppResult<()> {
    let updater = Updater::new(state_dir)?;
    match command {
        UpdateCommand::Check => match updater.check().await? {
            Some(release) => println!(
                "Update available: v{CURRENT} -> v{} ({})\n{}",
                release.version,
                if release.prerelease {
                    "prerelease"
                } else {
                    "stable"
                },
                release.url
            ),
            None => println!("homepage v{CURRENT} is up to date."),
        },
        UpdateCommand::Install { yes } => {
            let release = updater
                .check()
                .await?
                .ok_or_else(|| AppError::InvalidRequest("already up to date".into()))?;
            if !yes {
                return Err(AppError::InvalidRequest(format!(
                    "--yes is required to install v{} over v{CURRENT}",
                    release.version
                )));
            }
            let installed = updater.install(&release.version).await?;
            println!(
                "Installed v{} at {}.\nRestart the service to run it.",
                installed.version,
                installed.executable.display()
            );
        }
        UpdateCommand::Rollback { yes } => {
            if !yes {
                return Err(AppError::InvalidRequest(
                    "--yes is required to swap back to the previous executable".into(),
                ));
            }
            let executable = rollback()?;
            println!(
                "Restored the previous executable at {}; restart the service to run it.",
                executable.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn listing() -> Value {
        let asset = asset_name().unwrap_or_else(|| "none".into());
        json!([
            {"tag_name":"v0.5.0-beta.1","draft":false,"prerelease":true,"assets":[]},
            {"tag_name":"v0.4.1","draft":true,"prerelease":false,"assets":[]},
            {"tag_name":"v0.4.0","draft":false,"prerelease":false,"html_url":"https://example.com/r","body":"notes","assets":[
                {"name":"SHA256SUMS.txt","browser_download_url":"https://example.com/sums"},
                {"name":asset,"browser_download_url":"https://example.com/archive"}
            ]},
            {"tag_name":"v0.3.0","draft":false,"prerelease":true,"assets":[]},
            {"tag_name":"nightly","draft":false,"prerelease":true,"assets":[]}
        ])
    }

    #[test]
    fn selects_newest_by_channel_ignoring_drafts() {
        let pre = select(&listing(), "prerelease", "0.3.0").unwrap();
        assert_eq!(pre.version, "0.5.0-beta.1");
        let stable = select(&listing(), "stable", "0.3.0").unwrap();
        assert_eq!(stable.version, "0.4.0");
        assert_eq!(stable.sums_url.as_deref(), Some("https://example.com/sums"));
        if platform().is_some() {
            assert_eq!(
                stable.asset_url.as_deref(),
                Some("https://example.com/archive")
            );
        }
        assert!(select(&listing(), "stable", "0.4.0").is_none());
        assert!(select(&json!({"message":"rate limited"}), "stable", "0.3.0").is_none());
    }

    #[test]
    fn reads_sha256sums() {
        let sums = "ABC123  homepage-linux-x86_64.tar.gz\ndef456 *SHA256SUMS.txt\n";
        assert_eq!(
            expected_digest(sums, "homepage-linux-x86_64.tar.gz").as_deref(),
            Some("abc123")
        );
        assert_eq!(expected_digest(sums, SUMS).as_deref(), Some("def456"));
        assert!(expected_digest(sums, "homepage").is_none());
    }

    #[test]
    fn extracts_the_binary_from_a_release_archive() {
        let mut archive = Vec::new();
        {
            let encoder = flate2::write::GzEncoder::new(&mut archive, flate2::Compression::fast());
            let mut builder = tar::Builder::new(encoder);
            for (path, data) in [
                ("homepage-linux-x86_64/README.md", b"readme".as_slice()),
                ("homepage-linux-x86_64/homepage", b"binary".as_slice()),
            ] {
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o755);
                header.set_cksum();
                builder.append_data(&mut header, path, data).unwrap();
            }
            builder.into_inner().unwrap().finish().unwrap();
        }
        assert_eq!(extract_binary(&archive).unwrap(), b"binary");
        assert!(extract_binary(b"not an archive").is_err());
    }

    #[test]
    fn schedules_daily_and_weekly_slots() {
        let at = |text: &str| {
            Local
                .from_local_datetime(
                    &chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M").unwrap(),
                )
                .earliest()
                .unwrap()
        };
        let daily = Settings::default();
        // 2026-10-01 is a Thursday.
        let now = at("2026-10-01 10:00");
        assert!(due(&daily, None, now));
        assert!(due(&daily, Some(at("2026-10-01 03:59").timestamp()), now));
        assert!(!due(&daily, Some(at("2026-10-01 04:00").timestamp()), now));
        let weekly = Settings {
            schedule: "weekly".into(),
            weekday: 1,
            ..daily.clone()
        };
        assert!(!due(&weekly, Some(at("2026-09-28 04:30").timestamp()), now));
        assert!(due(&weekly, Some(at("2026-09-28 03:00").timestamp()), now));
        let off = Settings {
            schedule: "off".into(),
            ..daily
        };
        assert!(!due(&off, None, now));
    }

    #[test]
    fn validates_settings_and_persists_them() {
        assert!(Settings::default().validate().is_ok());
        for bad in [
            Settings {
                schedule: "hourly".into(),
                ..Settings::default()
            },
            Settings {
                weekday: 0,
                ..Settings::default()
            },
            Settings {
                time: "25:00".into(),
                ..Settings::default()
            },
            Settings {
                channel: "nightly".into(),
                ..Settings::default()
            },
        ] {
            assert!(bad.validate().is_err());
        }
        let dir = tempfile::tempdir().unwrap();
        let updater = Updater::new(&dir.path().join("state")).unwrap();
        let settings = Settings {
            schedule: "weekly".into(),
            ..Settings::default()
        };
        updater.save_settings(settings.clone()).unwrap();
        assert_eq!(updater.state().settings, settings);
        assert_eq!(updater.status()["available"], false);
    }

    #[test]
    fn swap_keeps_previous_binary() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("homepage");
        let staged = dir.path().join(".new");
        fs::write(&executable, "old").unwrap();
        fs::write(&staged, "new").unwrap();
        swap(&executable, &staged).unwrap();
        assert_eq!(fs::read_to_string(&executable).unwrap(), "new");
        assert_eq!(fs::read_to_string(old_path(&executable)).unwrap(), "old");
    }
}
