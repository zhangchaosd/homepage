//! Login and sessions, following obsidian-web-gateway: an Argon2 username and
//! password and/or a passkey from bookmarkd's database. Sessions are random
//! 256-bit tokens in an HttpOnly `SameSite=Strict` cookie, with a separate CSRF
//! token for every state-changing request. Unlike the gateway, login can never
//! be turned off.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    body::Body,
    http::{HeaderMap, Request},
    middleware::Next,
    response::Response,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;

use crate::{
    error::{AppError, AppResult},
    security::{
        passkey::PasskeyAuth,
        proxy::{self, TrustedProxy},
        sessions::{self, Session, SessionStore},
    },
};

pub const SESSION_COOKIE: &str = "hp_session";
/// Server-side lifetime of a session tied to the browser session.
const SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);
/// Absolute lifetime of a "keep me signed in" session.
pub const REMEMBER_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);
const FAILURE_WINDOW: Duration = Duration::from_secs(60);
const LOGIN_COOLDOWN: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct AuthStore {
    password_hash: Option<String>,
    username: Option<String>,
    passkey: Option<Arc<PasskeyAuth>>,
    secure_cookie: bool,
    trusted_proxies: Vec<TrustedProxy>,
    inner: Arc<Mutex<AuthInner>>,
}

#[derive(Default)]
struct AuthInner {
    sessions: SessionStore,
    failures: HashMap<String, Vec<Instant>>,
    next_attempt: HashMap<String, Instant>,
    active_attempts: HashSet<String>,
}

pub struct LoginResult {
    pub cookie: String,
    pub csrf: String,
}

impl AuthStore {
    pub fn new(password_hash: Option<String>, secure_cookie: bool) -> Self {
        Self {
            password_hash,
            username: None,
            passkey: None,
            secure_cookie,
            trusted_proxies: Vec::new(),
            inner: Arc::new(Mutex::new(AuthInner {
                sessions: SessionStore::ephemeral(),
                ..AuthInner::default()
            })),
        }
    }

    pub fn with_trusted_proxies(mut self, trusted_proxies: Vec<TrustedProxy>) -> Self {
        self.trusted_proxies = trusted_proxies;
        self
    }

    /// When set, password logins must also present this username.
    pub fn with_username(mut self, username: Option<String>) -> Self {
        self.username = username;
        self
    }

    pub fn with_passkey(mut self, passkey: Option<PasskeyAuth>) -> Self {
        self.passkey = passkey.map(Arc::new);
        self
    }

    /// Persists sessions at `path` so they survive restarts. `material`
    /// identifies the login configuration; changing it signs everyone out.
    pub fn with_session_file(self, path: &std::path::Path, material: &str) -> AppResult<Self> {
        self.lock()?.sessions = SessionStore::persistent(path, material)?;
        Ok(self)
    }

    pub fn password_enabled(&self) -> bool {
        self.password_hash.is_some()
    }
    pub fn username_required(&self) -> bool {
        self.password_enabled() && self.username.is_some()
    }
    pub fn passkey(&self) -> Option<Arc<PasskeyAuth>> {
        self.passkey.clone()
    }
    pub fn secure_cookie(&self) -> bool {
        self.secure_cookie
    }

    pub fn client_key(&self, peer: std::net::IpAddr, headers: &HeaderMap) -> String {
        proxy::client_ip(peer, headers, &self.trusted_proxies).to_string()
    }

    /// Rejects throttled clients and marks one attempt as in flight. Every
    /// successful call must be paired with [`AuthStore::end_attempt`].
    pub fn begin_attempt(&self, client: &str) -> AppResult<()> {
        let now = Instant::now();
        let mut inner = self.lock()?;
        let failures = inner.failures.entry(client.to_owned()).or_default();
        failures.retain(|attempt| now.duration_since(*attempt) < FAILURE_WINDOW);
        if failures.len() >= 5 {
            return Err(rate_limited(FAILURE_WINDOW));
        }
        if let Some(retry_at) = inner.next_attempt.get(client).copied() {
            if retry_at > now {
                return Err(rate_limited(retry_at.duration_since(now)));
            }
            inner.next_attempt.remove(client);
        }
        if !inner.active_attempts.insert(client.to_owned()) {
            return Err(rate_limited(LOGIN_COOLDOWN));
        }
        Ok(())
    }

    pub fn end_attempt(&self, client: &str, succeeded: bool) -> AppResult<()> {
        let at = Instant::now();
        let mut inner = self.lock()?;
        inner.active_attempts.remove(client);
        if succeeded {
            inner.failures.remove(client);
            inner.next_attempt.remove(client);
        } else {
            inner
                .failures
                .entry(client.to_owned())
                .or_default()
                .push(at);
            inner
                .next_attempt
                .insert(client.to_owned(), at + LOGIN_COOLDOWN);
        }
        Ok(())
    }

    pub fn login(
        &self,
        username: Option<&str>,
        password: &str,
        remember: bool,
        client: &str,
    ) -> AppResult<LoginResult> {
        let hash = self.password_hash.as_deref().ok_or(AppError::NotFound)?;
        let parsed = PasswordHash::new(hash)
            .map_err(|_| AppError::Internal("invalid password hash".into()))?;
        self.begin_attempt(client)?;
        // Always verify the password so a wrong username is not distinguishable by timing.
        let password_ok = Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
        let username_ok = self.username.as_deref().is_none_or(|expected| {
            bookmarkd_auth::secure_eq(
                &bookmarkd_auth::digest("username", expected),
                &bookmarkd_auth::digest("username", username.unwrap_or_default().trim()),
            )
        });
        let succeeded = password_ok && username_ok;
        self.end_attempt(client, succeeded)?;
        if !succeeded {
            return Err(AppError::Unauthenticated);
        }
        self.issue_session(remember, None)
    }

    /// Creates a session after any successful authentication method. Without
    /// `remember`, the cookie ends with the browser session and the server
    /// stops accepting it after 12 hours; with it, both last 30 days.
    pub fn issue_session(
        &self,
        remember: bool,
        credential: Option<String>,
    ) -> AppResult<LoginResult> {
        let token = random_token();
        let csrf = random_token();
        let ttl = if remember { REMEMBER_TTL } else { SESSION_TTL };
        self.lock()?.sessions.insert(
            &token,
            Session {
                csrf: csrf.clone(),
                expires: sessions::now() + ttl.as_secs() as i64,
                credential,
            },
        )?;
        let max_age = if remember {
            format!("; Max-Age={}", REMEMBER_TTL.as_secs())
        } else {
            String::new()
        };
        let secure = if self.secure_cookie { "; Secure" } else { "" };
        let cookie =
            format!("{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict{max_age}{secure}");
        Ok(LoginResult { cookie, csrf })
    }

    /// The session for this request, if it is current and its passkey (if
    /// any) has not been revoked.
    fn session(&self, headers: &HeaderMap) -> AppResult<Session> {
        let token = cookie_value(headers, SESSION_COOKIE).ok_or(AppError::Unauthenticated)?;
        let session = self
            .lock()?
            .sessions
            .get(token)
            .cloned()
            .ok_or(AppError::Unauthenticated)?;
        if let Some(credential) = &session.credential {
            let active = self
                .passkey()
                .is_some_and(|passkey| passkey.is_active(credential));
            if !active {
                return Err(AppError::Unauthenticated);
            }
        }
        Ok(session)
    }

    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, AuthInner>> {
        self.inner
            .lock()
            .map_err(|_| AppError::Internal("auth lock poisoned".into()))
    }

    pub fn logout(&self, headers: &HeaderMap) -> AppResult<()> {
        if let Some(token) = cookie_value(headers, SESSION_COOKIE) {
            self.lock()?.sessions.remove(token)?;
        }
        Ok(())
    }

    pub fn csrf_token(&self, headers: &HeaderMap) -> AppResult<String> {
        Ok(self.session(headers)?.csrf)
    }

    fn authorize(&self, headers: &HeaderMap, mutation: bool) -> AppResult<()> {
        let session = self.session(headers)?;
        if mutation {
            let csrf = headers
                .get("x-csrf-token")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            if !bookmarkd_auth::secure_eq(csrf, &session.csrf) {
                return Err(AppError::Forbidden);
            }
        }
        Ok(())
    }
}

fn rate_limited(retry_after: Duration) -> AppError {
    AppError::RateLimited {
        retry_after_seconds: retry_after.as_secs().max(1),
    }
}

pub async fn require_auth(
    axum::extract::State(auth): axum::extract::State<AuthStore>,
    request: Request<Body>,
    next: Next,
) -> AppResult<Response> {
    let mutation = !matches!(
        *request.method(),
        http::Method::GET | http::Method::HEAD | http::Method::OPTIONS
    );
    auth.authorize(request.headers(), mutation)?;
    Ok(next.run(request).await)
}

pub fn cookie_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(http::header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .find_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == name).then_some(value)
        })
}

pub fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use argon2::{PasswordHasher, password_hash::SaltString};
    use http::HeaderValue;

    fn hash(password: &str) -> String {
        let salt = SaltString::encode_b64(b"0123456789abcdef").unwrap();
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }

    fn cookie_header(login: &LoginResult) -> HeaderMap {
        let pair = login.cookie.split(';').next().expect("cookie pair");
        let mut headers = HeaderMap::new();
        headers.insert(http::header::COOKIE, HeaderValue::from_str(pair).unwrap());
        headers
    }

    #[test]
    fn session_requires_csrf_for_mutations() {
        let auth = AuthStore::new(Some(hash("correct horse")), false);
        assert!(auth.login(None, "wrong", false, "client").is_err());
        assert!(matches!(
            auth.login(None, "correct horse", false, "client"),
            Err(AppError::RateLimited { .. })
        ));
        std::thread::sleep(LOGIN_COOLDOWN);
        let login = auth.login(None, "correct horse", false, "client").unwrap();
        let mut headers = cookie_header(&login);
        assert!(auth.authorize(&headers, false).is_ok());
        assert!(auth.authorize(&headers, true).is_err());
        headers.insert("x-csrf-token", HeaderValue::from_str(&login.csrf).unwrap());
        assert!(auth.authorize(&headers, true).is_ok());
        assert!(auth.authorize(&HeaderMap::new(), false).is_err());
    }

    #[test]
    fn configured_username_is_required() {
        let auth =
            AuthStore::new(Some(hash("correct horse")), true).with_username(Some("chao".into()));
        assert!(auth.username_required());
        assert!(matches!(
            auth.login(Some("other"), "correct horse", false, "client"),
            Err(AppError::Unauthenticated)
        ));
        std::thread::sleep(LOGIN_COOLDOWN);
        let login = auth
            .login(Some(" chao "), "correct horse", true, "client")
            .unwrap();
        assert!(login.cookie.contains("Max-Age=2592000") && login.cookie.contains("Secure"));
    }

    #[test]
    fn sessions_survive_restart_until_settings_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        let auth = AuthStore::new(None, true)
            .with_session_file(&path, "a")
            .unwrap();
        let login = auth.issue_session(false, None).unwrap();
        let restarted = AuthStore::new(None, true)
            .with_session_file(&path, "a")
            .unwrap();
        assert!(restarted.authorize(&cookie_header(&login), false).is_ok());
        let changed = AuthStore::new(None, true)
            .with_session_file(&path, "b")
            .unwrap();
        assert!(changed.authorize(&cookie_header(&login), false).is_err());
    }

    #[test]
    fn password_login_is_unavailable_without_a_hash() {
        let auth = AuthStore::new(None, false);
        assert!(matches!(
            auth.login(None, "x", false, "c"),
            Err(AppError::NotFound)
        ));
    }
}
