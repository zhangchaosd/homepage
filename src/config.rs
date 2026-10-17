//! Configuration lives in one directory:
//!
//! - `config.toml`  — everything that is not secret (services, widgets, devices)
//! - `secrets.toml` — password hash and API tokens; keep it mode 600
//! - `auth.db`      — passkey database copied from bookmarkd (optional)
//! - `icons/`       — custom service icons served at `/icons/<file>` (optional)
//! - `state/`       — written by the server (sessions); created on demand

use std::{
    collections::HashMap,
    env, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    security::proxy::TrustedProxy,
    update::UpdateCommand,
};

#[derive(Debug, Parser)]
#[command(name = "homepage", version, about = "Private personal start page")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Directory holding config.toml, secrets.toml and server state.
    #[arg(
        long,
        short = 'c',
        value_name = "DIR",
        env = "HOMEPAGE_CONFIG_DIR",
        default_value = "config"
    )]
    config_dir: PathBuf,
    /// Overrides `server.listen`.
    #[arg(long, value_name = "IP:PORT")]
    listen: Option<SocketAddr>,
    #[arg(long, env = "HOMEPAGE_LOG", default_value = "info")]
    log_level: String,
}

#[derive(Clone, Copy, Debug, Subcommand)]
enum Command {
    /// Prompt for a password and print its Argon2 hash for secrets.toml.
    HashPassword,
    /// Load and validate the configuration directory, then exit.
    CheckConfig,
    /// Check GitHub Releases for a newer version, install it, or roll back.
    Update {
        #[command(subcommand)]
        command: UpdateCommand,
    },
}

pub enum Launch {
    Serve(Box<Config>),
    HashPassword,
    CheckConfig(Box<Config>),
    Update {
        command: UpdateCommand,
        state_dir: PathBuf,
    },
}

// ---------------------------------------------------------------- file format

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    server: ServerSection,
    #[serde(default)]
    auth: AuthSection,
    #[serde(default)]
    profile: Profile,
    #[serde(default)]
    services: Vec<Service>,
    #[serde(default)]
    devices: Vec<DeviceSection>,
    vikunja: Option<VikunjaSection>,
    forgejo: Option<ForgejoSection>,
    github: Option<GithubSection>,
    bookmarkd: Option<BookmarkdSection>,
    homeassistant: Option<HomeAssistantSection>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServerSection {
    listen: Option<SocketAddr>,
    public_url: String,
    #[serde(default)]
    trusted_proxies: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthSection {
    username: Option<String>,
    passkey_db: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tagline: String,
    /// Page title in the browser tab.
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub name: String,
    /// Link opened in the browser (usually the public URL).
    pub url: String,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Built-in icon name (forgejo, vikunja, homeassistant, obsidian, bookmark, github, …)
    /// or a file in `<config>/icons/`.
    #[serde(default)]
    pub icon: Option<String>,
    /// URL probed for the status dot, typically the LAN address. Defaults to `url`;
    /// set to `""` to disable probing.
    #[serde(default, skip_serializing)]
    pub check: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceSection {
    name: String,
    /// ipv6who base URL, e.g. http://192.168.1.10:8066
    url: String,
    #[serde(default)]
    kind: Option<String>,
    /// Key in secrets.toml `[tokens]`; defaults to "ipv6who" when that key exists.
    #[serde(default)]
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VikunjaSection {
    url: String,
    link: Option<String>,
    token: Option<String>,
    /// How many days ahead count as "upcoming".
    #[serde(default = "default_upcoming_days")]
    upcoming_days: u32,
    refresh: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForgejoSection {
    url: String,
    link: Option<String>,
    username: String,
    token: Option<String>,
    #[serde(default = "default_commit_limit")]
    limit: usize,
    refresh: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GithubSection {
    username: String,
    token: Option<String>,
    #[serde(default = "default_repo_limit")]
    limit: usize,
    refresh: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BookmarkdSection {
    url: String,
    link: Option<String>,
    token: Option<String>,
    #[serde(default = "default_bookmark_limit")]
    limit: usize,
    refresh: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HomeAssistantSection {
    url: String,
    link: Option<String>,
    token: Option<String>,
    #[serde(default)]
    entities: Vec<HaEntity>,
    refresh: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HaEntity {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SecretsFile {
    password_hash: Option<String>,
    #[serde(default)]
    tokens: HashMap<String, String>,
}

fn default_upcoming_days() -> u32 {
    7
}
fn default_commit_limit() -> usize {
    8
}
fn default_repo_limit() -> usize {
    6
}
fn default_bookmark_limit() -> usize {
    12
}

// ------------------------------------------------------------- resolved config

#[derive(Clone, Debug)]
pub struct Config {
    pub dir: PathBuf,
    pub state_dir: PathBuf,
    pub listen: SocketAddr,
    pub log_level: String,
    pub public_url: String,
    pub secure_cookie: bool,
    pub trusted_proxies: Vec<TrustedProxy>,
    pub username: Option<String>,
    pub password_hash: Option<String>,
    pub passkey_db: Option<PathBuf>,
    pub profile: Profile,
    pub services: Vec<Service>,
    pub devices: Vec<Device>,
    pub vikunja: Option<Vikunja>,
    pub forgejo: Option<Forgejo>,
    pub github: Option<Github>,
    pub bookmarkd: Option<Bookmarkd>,
    pub homeassistant: Option<HomeAssistant>,
}

#[derive(Clone, Debug)]
pub struct Device {
    pub name: String,
    pub url: String,
    pub kind: String,
    pub token: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Vikunja {
    pub url: String,
    pub link: String,
    pub token: String,
    pub upcoming_days: u32,
    pub refresh: Duration,
}

#[derive(Clone, Debug)]
pub struct Forgejo {
    pub url: String,
    pub link: String,
    pub username: String,
    pub token: Option<String>,
    pub limit: usize,
    pub refresh: Duration,
}

#[derive(Clone, Debug)]
pub struct Github {
    pub username: String,
    pub token: Option<String>,
    pub limit: usize,
    pub refresh: Duration,
}

#[derive(Clone, Debug)]
pub struct Bookmarkd {
    pub url: String,
    pub link: String,
    pub token: String,
    pub limit: usize,
    pub refresh: Duration,
}

#[derive(Clone, Debug)]
pub struct HomeAssistant {
    pub url: String,
    pub link: String,
    pub token: String,
    pub entities: Vec<HaEntity>,
    pub refresh: Duration,
}

impl Config {
    pub fn launch() -> AppResult<Launch> {
        let cli = Cli::parse();
        match cli.command {
            Some(Command::HashPassword) => return Ok(Launch::HashPassword),
            // Updating needs only the state directory, not a valid configuration.
            Some(Command::Update { command }) => {
                return Ok(Launch::Update {
                    command,
                    state_dir: cli.config_dir.join("state"),
                });
            }
            _ => {}
        }
        let config = Self::load(&cli.config_dir, cli.listen, cli.log_level)?;
        Ok(match cli.command {
            Some(Command::CheckConfig) => Launch::CheckConfig(Box::new(config)),
            _ => Launch::Serve(Box::new(config)),
        })
    }

    pub fn load(dir: &Path, listen: Option<SocketAddr>, log_level: String) -> AppResult<Self> {
        let invalid = |message: String| AppError::InvalidRequest(message);
        let config_path = dir.join("config.toml");
        let text = fs::read_to_string(&config_path)
            .map_err(|error| invalid(format!("cannot read {}: {error}", config_path.display())))?;
        let file: FileConfig = toml::from_str(&text)
            .map_err(|error| invalid(format!("invalid {}: {error}", config_path.display())))?;
        let secrets = load_secrets(&dir.join("secrets.toml"))?;

        let public_url = file.server.public_url.trim_end_matches('/').to_owned();
        validate_public_url(&public_url)?;
        let trusted_proxies = file
            .server
            .trusted_proxies
            .iter()
            .map(|value| TrustedProxy::parse(value))
            .collect::<AppResult<Vec<_>>>()?;

        let password_hash = env::var("HOMEPAGE_PASSWORD_HASH")
            .ok()
            .or(secrets.password_hash)
            .filter(|value| !value.trim().is_empty());
        if let Some(hash) = &password_hash {
            argon2::PasswordHash::new(hash).map_err(|_| {
                invalid("password_hash is not an Argon2 hash; generate one with `homepage hash-password`".into())
            })?;
        }
        let passkey_db = file.auth.passkey_db.map(|path| resolve(dir, &path));
        if password_hash.is_none() && passkey_db.is_none() {
            return Err(invalid(
                "no login method: set password_hash in secrets.toml and/or auth.passkey_db in config.toml"
                    .into(),
            ));
        }
        let username = file
            .auth
            .username
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        let tokens = secrets.tokens;
        let token = |name: Option<&str>, default: &str| -> Option<String> {
            tokens.get(name.unwrap_or(default)).cloned()
        };
        let required = |name: Option<&str>, default: &str| -> AppResult<String> {
            let key = name.unwrap_or(default);
            tokens.get(key).cloned().ok_or_else(|| {
                invalid(format!(
                    "[{default}] needs a token: add `{key} = \"…\"` under [tokens] in secrets.toml"
                ))
            })
        };
        let refresh = |value: Option<u64>, default: u64| {
            Duration::from_secs(value.unwrap_or(default).max(10))
        };

        let devices = file
            .devices
            .into_iter()
            .map(|device| {
                if let Some(key) = &device.token
                    && !tokens.contains_key(key)
                {
                    return Err(invalid(format!(
                        "device {} uses token `{key}`, which is missing from secrets.toml",
                        device.name
                    )));
                }
                Ok(Device {
                    token: token(device.token.as_deref(), "ipv6who"),
                    url: device.url.trim_end_matches('/').to_owned(),
                    kind: device.kind.unwrap_or_else(|| "server".into()),
                    name: device.name,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        let vikunja = file
            .vikunja
            .map(|section| -> AppResult<Vikunja> {
                let url = base_url(&section.url);
                Ok(Vikunja {
                    token: required(section.token.as_deref(), "vikunja")?,
                    link: section
                        .link
                        .map_or_else(|| url.clone(), |link| base_url(&link)),
                    url,
                    upcoming_days: section.upcoming_days.min(60),
                    refresh: refresh(section.refresh, 60),
                })
            })
            .transpose()?;
        let forgejo = file.forgejo.map(|section| {
            let url = base_url(&section.url);
            Forgejo {
                token: token(section.token.as_deref(), "forgejo"),
                link: section
                    .link
                    .map_or_else(|| url.clone(), |link| base_url(&link)),
                url,
                username: section.username,
                limit: section.limit.clamp(1, 30),
                refresh: refresh(section.refresh, 120),
            }
        });
        let github = file.github.map(|section| Github {
            token: token(section.token.as_deref(), "github"),
            username: section.username,
            limit: section.limit.clamp(1, 12),
            refresh: refresh(section.refresh, 900),
        });
        let bookmarkd = file
            .bookmarkd
            .map(|section| -> AppResult<Bookmarkd> {
                let url = base_url(&section.url);
                Ok(Bookmarkd {
                    token: required(section.token.as_deref(), "bookmarkd")?,
                    link: section
                        .link
                        .map_or_else(|| url.clone(), |link| base_url(&link)),
                    url,
                    limit: section.limit.clamp(1, 48),
                    refresh: refresh(section.refresh, 300),
                })
            })
            .transpose()?;
        let homeassistant = file
            .homeassistant
            .map(|section| -> AppResult<HomeAssistant> {
                let url = base_url(&section.url);
                Ok(HomeAssistant {
                    token: required(section.token.as_deref(), "homeassistant")?,
                    link: section
                        .link
                        .map_or_else(|| url.clone(), |link| base_url(&link)),
                    url,
                    entities: section.entities,
                    refresh: refresh(section.refresh, 30),
                })
            })
            .transpose()?;

        Ok(Self {
            dir: dir.to_path_buf(),
            state_dir: dir.join("state"),
            listen: listen
                .or(file.server.listen)
                .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8780))),
            log_level,
            secure_cookie: public_url.starts_with("https://"),
            public_url,
            trusted_proxies,
            username,
            password_hash,
            passkey_db,
            profile: file.profile,
            services: file.services,
            devices,
            vikunja,
            forgejo,
            github,
            bookmarkd,
            homeassistant,
        })
    }
}

fn load_secrets(path: &Path) -> AppResult<SecretsFile> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(SecretsFile::default());
        }
        Err(error) => {
            return Err(AppError::InvalidRequest(format!(
                "cannot read {}: {error}",
                path.display()
            )));
        }
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path)
            && metadata.permissions().mode() & 0o077 != 0
        {
            // Tracing is not set up yet while the config loads.
            eprintln!(
                "homepage: warning: {} is readable by other users; run chmod 600",
                path.display()
            );
        }
    }
    toml::from_str(&text)
        .map_err(|error| AppError::InvalidRequest(format!("invalid {}: {error}", path.display())))
}

fn resolve(dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        dir.join(path)
    }
}

fn base_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_owned()
}

/// Passkeys are bound to an exact origin, so it must be written exactly as browsers send it.
fn validate_public_url(origin: &str) -> AppResult<()> {
    let invalid =
        |reason: &str| AppError::InvalidRequest(format!("invalid server.public_url: {reason}"));
    let url = url::Url::parse(origin).map_err(|error| invalid(&error.to_string()))?;
    if url.origin().ascii_serialization() != origin {
        return Err(invalid(
            "use only scheme, host, and optional port, without a path or trailing slash",
        ));
    }
    let local = url.host_str() == Some("localhost");
    if url.scheme() != "https" && !(url.scheme() == "http" && local) {
        return Err(invalid("HTTPS is required except for http://localhost"));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
[server]
public_url = "https://home.example.com"

[auth]
passkey_db = "auth.db"

[[services]]
name = "Forgejo"
url = "https://git.example.com"
check = "http://192.168.1.10:3000"

[[devices]]
name = "NAS"
url = "http://192.168.1.10:8066/"

[vikunja]
url = "http://192.168.1.10:3456/"
link = "https://todo.example.com"
"#;

    fn write(dir: &Path, config: &str, secrets: &str) {
        fs::write(dir.join("config.toml"), config).unwrap();
        fs::write(dir.join("secrets.toml"), secrets).unwrap();
    }

    #[test]
    fn resolves_paths_tokens_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            MINIMAL,
            "[tokens]\nvikunja = \"tk_1\"\nipv6who = \"v6\"\n",
        );
        let config = Config::load(dir.path(), None, "info".into()).unwrap();
        assert_eq!(config.passkey_db.unwrap(), dir.path().join("auth.db"));
        assert!(config.secure_cookie);
        assert_eq!(config.devices[0].url, "http://192.168.1.10:8066");
        assert_eq!(config.devices[0].token.as_deref(), Some("v6"));
        let vikunja = config.vikunja.unwrap();
        assert_eq!(vikunja.url, "http://192.168.1.10:3456");
        assert_eq!(vikunja.token, "tk_1");
        assert_eq!(vikunja.upcoming_days, 7);
    }

    #[test]
    fn missing_required_token_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), MINIMAL, "");
        let error = Config::load(dir.path(), None, "info".into()).unwrap_err();
        assert!(error.to_string().contains("vikunja"), "{error}");
    }

    #[test]
    fn requires_a_login_method_and_exact_origin() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "[server]\npublic_url = \"https://home.example.com\"\n",
            "",
        );
        assert!(Config::load(dir.path(), None, "info".into()).is_err());
        write(
            dir.path(),
            "[server]\npublic_url = \"https://home.example.com/\"\n[auth]\npasskey_db = \"a.db\"\n",
            "",
        );
        // A trailing slash is trimmed; a path is not allowed.
        assert!(Config::load(dir.path(), None, "info".into()).is_ok());
        write(
            dir.path(),
            "[server]\npublic_url = \"http://home.example.com\"\n[auth]\npasskey_db = \"a.db\"\n",
            "",
        );
        assert!(Config::load(dir.path(), None, "info".into()).is_err());
    }
}
