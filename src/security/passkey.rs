//! Passkey login backed by bookmarkd's authentication store.
//!
//! The SQLite database written by bookmarkd holds the WebAuthn identity (RP ID
//! and user handle) and the registered credentials. Copying it here, or pointing
//! at the same file on one host, lets the same passkeys sign in to the homepage
//! as long as both sites live under that RP ID. Credentials are managed in
//! bookmarkd; the homepage only verifies them.

use std::{
    collections::HashMap,
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};

use bookmarkd_auth::{
    AuthStore as _, SqliteAuthStore, StoredCredential,
    engine::{CeremonyState, Engine},
};
use serde_json::Value;

use crate::{
    error::{AppError, AppResult},
    security::auth::random_token,
};

/// Scope for the homepage's rows in the shared store; never bookmarkd's own.
pub const APP_ID: &str = "homepage";
const CEREMONY_TTL: Duration = Duration::from_secs(300);
const MAX_CEREMONIES: usize = 1000;
const MAX_CEREMONIES_PER_BROWSER: usize = 5;
/// How long a revocation in the shared database may take to end sessions.
const ACTIVE_CACHE_TTL: Duration = Duration::from_secs(30);

pub struct PasskeyAuth {
    store: SqliteAuthStore,
    engine: Engine,
    origin: String,
    rp_id: String,
    ceremonies: Mutex<HashMap<String, Ceremony>>,
    active: Mutex<Option<(Instant, std::collections::HashSet<String>)>>,
}

struct Ceremony {
    state: CeremonyState,
    binding: String,
    expires: Instant,
    credentials: Vec<StoredCredential>,
}

impl PasskeyAuth {
    pub fn open(path: &Path, origin: &str, user_name: &str) -> AppResult<Self> {
        let invalid = |message: String| AppError::InvalidRequest(message);
        if !path.is_file() {
            return Err(invalid(format!(
                "passkey database not found: {}",
                path.display()
            )));
        }
        let store = SqliteAuthStore {
            path: path.to_path_buf(),
        };
        let identity = store.identity().map_err(|error| {
            invalid(format!(
                "cannot read passkey database {}: {error:#}",
                path.display()
            ))
        })?;
        let host = url::Url::parse(origin)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .ok_or_else(|| invalid("invalid server.public_url".into()))?;
        let rp_id = identity.rp_id.clone();
        if host != rp_id && !host.ends_with(&format!(".{rp_id}")) {
            return Err(invalid(format!(
                "server.public_url host {host} is not within the passkey RP ID {rp_id}; serve the homepage from {rp_id} or one of its subdomains"
            )));
        }
        let engine = Engine::new(&identity, "Homepage", user_name, origin)
            .map_err(|error| invalid(format!("invalid passkey identity: {error:#}")))?;
        Ok(Self {
            store,
            engine,
            origin: origin.to_owned(),
            rp_id,
            ceremonies: Mutex::new(HashMap::new()),
            active: Mutex::new(None),
        })
    }

    pub fn rp_id(&self) -> &str {
        &self.rp_id
    }

    pub fn credential_count(&self) -> AppResult<usize> {
        Ok(self.credentials()?.len())
    }

    /// Starts a login ceremony bound to the browser's `binding` cookie value.
    /// Returns WebAuthn request options with a `ceremonyId` for the finish call.
    pub fn begin(&self, binding: &str) -> AppResult<Value> {
        let credentials = self.credentials()?;
        if credentials.is_empty() {
            return Err(AppError::InvalidRequest(
                "no passkeys are registered; register one in bookmarkd and copy its database again"
                    .into(),
            ));
        }
        let (mut options, state) = self
            .engine
            .login_begin(&credentials)
            .map_err(|error| AppError::Internal(format!("passkey challenge failed: {error:#}")))?;
        let id = random_token();
        let now = Instant::now();
        let mut ceremonies = self.lock()?;
        ceremonies.retain(|_, ceremony| ceremony.expires > now);
        if ceremonies.len() >= MAX_CEREMONIES
            || ceremonies
                .values()
                .filter(|ceremony| ceremony.binding == binding)
                .count()
                >= MAX_CEREMONIES_PER_BROWSER
        {
            return Err(AppError::RateLimited {
                retry_after_seconds: 5,
            });
        }
        ceremonies.insert(
            id.clone(),
            Ceremony {
                state,
                binding: binding.to_owned(),
                expires: now + CEREMONY_TTL,
                credentials,
            },
        );
        options["ceremonyId"] = Value::String(id);
        Ok(options)
    }

    /// Whether a credential is still registered and not revoked. Cached briefly;
    /// if the database is unavailable the last known list is kept.
    pub fn is_active(&self, credential_id: &str) -> bool {
        let Ok(mut active) = self.active.lock() else {
            return false;
        };
        let stale = active
            .as_ref()
            .is_none_or(|(checked, _)| checked.elapsed() > ACTIVE_CACHE_TTL);
        if stale {
            let previous = active.take().map(|(_, ids)| ids).unwrap_or_default();
            let ids = self.credentials().map_or(previous, |credentials| {
                credentials
                    .into_iter()
                    .map(|credential| credential.id)
                    .collect()
            });
            *active = Some((Instant::now(), ids));
        }
        active
            .as_ref()
            .is_some_and(|(_, ids)| ids.contains(credential_id))
    }

    /// Verifies the authenticator response and returns the credential ID.
    /// Ceremony state is consumed before verification so a response can never
    /// be replayed.
    pub fn finish(&self, ceremony_id: &str, binding: &str, credential: Value) -> AppResult<String> {
        let ceremony = self
            .lock()?
            .remove(ceremony_id)
            .ok_or(AppError::Unauthenticated)?;
        if ceremony.expires <= Instant::now()
            || !bookmarkd_auth::secure_eq(&ceremony.binding, binding)
        {
            return Err(AppError::Unauthenticated);
        }
        let updated = self
            .engine
            .login_finish(credential, ceremony.state, &ceremony.credentials)
            .map_err(|error| {
                tracing::warn!(error = %error, "passkey verification failed");
                AppError::Unauthenticated
            })?;
        // Persists the signature counter and backup flags. It also records a
        // session row scoped to this app, which expires on its own.
        self.store
            .finish_login(&updated, APP_ID, &self.origin, false)
            .map_err(|error| {
                tracing::warn!(error = %error, "passkey credential update failed");
                AppError::Unauthenticated
            })?;
        Ok(updated.id)
    }

    fn credentials(&self) -> AppResult<Vec<StoredCredential>> {
        self.store
            .credentials()
            .map_err(|error| AppError::Internal(format!("passkey database unavailable: {error:#}")))
    }

    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, HashMap<String, Ceremony>>> {
        self.ceremonies
            .lock()
            .map_err(|_| AppError::Internal("passkey lock poisoned".into()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use bookmarkd_auth::Identity;

    fn database(rp_id: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("auth.db");
        SqliteAuthStore::initialize(
            &path,
            &Identity {
                rp_id: rp_id.into(),
                user_id: bookmarkd_auth::random(),
            },
        )
        .expect("initialize");
        (dir, path)
    }

    #[test]
    fn accepts_subdomains_of_the_shared_rp_id() {
        let (_dir, path) = database("example.com");
        let passkey = PasskeyAuth::open(&path, "https://home.example.com", "owner").expect("open");
        assert_eq!(passkey.rp_id(), "example.com");
        assert_eq!(passkey.credential_count().expect("count"), 0);
        assert!(matches!(
            passkey.begin("browser"),
            Err(AppError::InvalidRequest(_))
        ));
    }

    #[test]
    fn rejects_foreign_origins_missing_databases_and_unknown_ceremonies() {
        let (dir, path) = database("example.com");
        assert!(PasskeyAuth::open(&path, "https://example.org", "owner").is_err());
        assert!(PasskeyAuth::open(&path, "https://badexample.com", "owner").is_err());
        assert!(
            PasskeyAuth::open(
                &dir.path().join("missing.db"),
                "https://example.com",
                "owner"
            )
            .is_err()
        );
        let passkey = PasskeyAuth::open(&path, "https://example.com", "owner").expect("open");
        assert!(matches!(
            passkey.finish("missing", "browser", serde_json::json!({})),
            Err(AppError::Unauthenticated)
        ));
    }
}
