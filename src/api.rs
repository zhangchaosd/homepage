use std::{collections::HashMap, net::SocketAddr};

use axum::{
    Json,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    config::{Profile, Service},
    error::{AppError, AppResult},
    security::auth::{LoginResult, SESSION_COOKIE, cookie_value, random_token},
    update,
    widgets::{Slot, Widget, inbox, vikunja},
};

// ------------------------------------------------------------------- public

#[derive(Serialize)]
pub struct SystemResponse {
    version: &'static str,
    title: String,
    auth: AuthMethods,
}

#[derive(Serialize)]
pub struct AuthMethods {
    password: bool,
    username: bool,
    passkey: bool,
}

/// What the login page needs; deliberately nothing about the dashboard.
pub async fn system(State(state): State<AppState>) -> Json<SystemResponse> {
    let profile = &state.hub.config().profile;
    Json(SystemResponse {
        version: env!("CARGO_PKG_VERSION"),
        title: profile.title.clone().unwrap_or_else(|| "Home".into()),
        auth: AuthMethods {
            password: state.auth.password_enabled(),
            username: state.auth.username_required(),
            passkey: state.auth.passkey().is_some(),
        },
    })
}

pub async fn health() -> &'static str {
    "ok"
}

#[derive(Deserialize)]
pub struct LoginRequest {
    #[serde(default)]
    username: Option<String>,
    password: String,
    #[serde(default)]
    remember: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    csrf_token: String,
}

pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> AppResult<Response> {
    let store = state.auth.clone();
    let client = store.client_key(peer.ip(), &headers);
    let result = tokio::task::spawn_blocking(move || {
        store.login(
            request.username.as_deref(),
            &request.password,
            request.remember,
            &client,
        )
    })
    .await??;
    session_response(result)
}

fn session_response(result: LoginResult) -> AppResult<Response> {
    let mut response = Json(SessionResponse {
        csrf_token: result.csrf,
    })
    .into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&result.cookie)
            .map_err(|_| AppError::Internal("invalid session cookie".into()))?,
    );
    Ok(response)
}

/// Random per-browser value tying a passkey ceremony to the browser that started it.
const PASSKEY_BINDING_COOKIE: &str = "hp_passkey";

pub async fn passkey_begin(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let passkey = state.auth.passkey().ok_or(AppError::NotFound)?;
    let client = state.auth.client_key(peer.ip(), &headers);
    let binding = cookie_value(&headers, PASSKEY_BINDING_COOKIE)
        .filter(|value| value.len() == 43)
        .map_or_else(random_token, str::to_owned);
    let auth = state.auth.clone();
    let (options, binding) = tokio::task::spawn_blocking(move || {
        auth.begin_attempt(&client)?;
        // Starting a ceremony is not a failure; release the slot immediately.
        auth.end_attempt(&client, true)?;
        passkey.begin(&binding).map(|options| (options, binding))
    })
    .await??;
    let mut response = Json(options).into_response();
    let secure = if state.auth.secure_cookie() {
        "; Secure"
    } else {
        ""
    };
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "{PASSKEY_BINDING_COOKIE}={binding}; Path=/api/v1/auth/passkey; HttpOnly; SameSite=Strict; Max-Age=600{secure}"
        ))
        .map_err(|_| AppError::Internal("invalid passkey cookie".into()))?,
    );
    Ok(response)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyFinishRequest {
    ceremony_id: String,
    credential: serde_json::Value,
    #[serde(default)]
    remember: bool,
}

pub async fn passkey_finish(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<PasskeyFinishRequest>,
) -> AppResult<Response> {
    let passkey = state.auth.passkey().ok_or(AppError::NotFound)?;
    let client = state.auth.client_key(peer.ip(), &headers);
    let binding = cookie_value(&headers, PASSKEY_BINDING_COOKIE)
        .unwrap_or_default()
        .to_owned();
    let auth = state.auth.clone();
    let result = tokio::task::spawn_blocking(move || {
        auth.begin_attempt(&client)?;
        let verified = passkey.finish(&request.ceremony_id, &binding, request.credential);
        auth.end_attempt(&client, verified.is_ok())?;
        auth.issue_session(request.remember, Some(verified?))
    })
    .await??;
    session_response(result)
}

// ---------------------------------------------------------------- protected

pub async fn auth_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<SessionResponse>> {
    Ok(Json(SessionResponse {
        csrf_token: state.auth.csrf_token(&headers)?,
    }))
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    state.auth.logout(&headers)?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"
        ))
        .map_err(|_| AppError::Internal("invalid cookie".into()))?,
    );
    Ok(response)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardResponse {
    profile: Profile,
    services: Vec<Service>,
    links: HashMap<&'static str, String>,
    widgets: HashMap<&'static str, Slot>,
    server_time: i64,
}

pub async fn dashboard(State(state): State<AppState>) -> Json<DashboardResponse> {
    let config = state.hub.config();
    let mut links = HashMap::new();
    if let Some(section) = &config.vikunja {
        links.insert("vikunja", section.link.clone());
    }
    if let Some(section) = &config.forgejo {
        links.insert("forgejo", section.link.clone());
    }
    if let Some(section) = &config.github {
        links.insert("github", format!("https://github.com/{}", section.username));
    }
    if let Some(section) = &config.bookmarkd {
        links.insert("bookmarkd", section.link.clone());
    }
    if let Some(section) = &config.homeassistant {
        links.insert("homeassistant", section.link.clone());
    }
    if let Some(section) = &config.inbox {
        links.insert("inbox", section.link.clone());
    }
    Json(DashboardResponse {
        profile: config.profile.clone(),
        services: config.services.clone(),
        links,
        widgets: state.hub.snapshot().await,
        server_time: chrono::Utc::now().timestamp(),
    })
}

pub async fn refresh(
    State(state): State<AppState>,
    Path(widget): Path<String>,
) -> AppResult<StatusCode> {
    let widget = Widget::parse(&widget).ok_or(AppError::NotFound)?;
    if state.hub.wake(widget) {
        Ok(StatusCode::ACCEPTED)
    } else {
        Err(AppError::NotFound)
    }
}

pub async fn complete_todo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let config = state
        .hub
        .config()
        .vikunja
        .as_ref()
        .ok_or(AppError::NotFound)?;
    vikunja::complete(&state.hub.client, config, id)
        .await
        .map_err(AppError::Upstream)?;
    state.hub.wake(Widget::Vikunja);
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct InboxSendRequest {
    #[serde(default)]
    title: Option<String>,
    text: String,
}

/// Quick send to the cross-device inbox with its write-only token.
pub async fn inbox_send(
    State(state): State<AppState>,
    Json(request): Json<InboxSendRequest>,
) -> AppResult<Json<inbox::Receipt>> {
    let config = state
        .hub
        .config()
        .inbox
        .as_ref()
        .ok_or(AppError::NotFound)?;
    let receipt = inbox::send(
        &state.hub.client,
        config,
        request.title.as_deref(),
        &request.text,
    )
    .await
    .map_err(AppError::Upstream)?;
    // The new entry should show up in the recent list right away.
    state.hub.wake(Widget::Inbox);
    Ok(Json(receipt))
}

/// Custom service icons from `<config>/icons/`. Names are restricted to a
/// plain file name with an image extension.
pub async fn icon(State(state): State<AppState>, Path(name): Path<String>) -> AppResult<Response> {
    let valid_name = !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    let mime = mime_guess::from_path(&name).first_or_octet_stream();
    if !valid_name || mime.type_() != mime_guess::mime::IMAGE {
        return Err(AppError::NotFound);
    }
    let path = state.hub.config().dir.join("icons").join(&name);
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| AppError::NotFound)?;
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref())
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=3600"),
    );
    if mime.subtype() == "svg" {
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'none'; style-src 'unsafe-inline'; sandbox"),
        );
    }
    Ok(response)
}

// ------------------------------------------------------------------ updates

pub async fn update_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(state.updater.status())
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(settings): Json<update::Settings>,
) -> AppResult<Json<serde_json::Value>> {
    state.updater.save_settings(settings)?;
    Ok(Json(state.updater.status()))
}

pub async fn update_check(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let recently = state
        .updater
        .last_checked()
        .is_some_and(|checked| chrono::Utc::now().timestamp() - checked < 10);
    if recently {
        return Err(AppError::InvalidRequest(
            "updates were checked moments ago; try again in a few seconds".into(),
        ));
    }
    // A failed check is recorded in the status, which the UI shows.
    let _ = state.updater.check().await;
    Ok(Json(state.updater.status()))
}

#[derive(Deserialize)]
pub struct InstallRequest {
    version: String,
}

pub async fn update_install(
    State(state): State<AppState>,
    Json(request): Json<InstallRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let installed = state.updater.install(&request.version).await?;
    tracing::info!(version = %installed.version, "update installed; restarting");
    #[cfg(unix)]
    {
        // Let this response reach the browser before the process image is replaced.
        let executable = installed.executable.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(800));
            update::reexec(&executable)
        });
    }
    Ok(Json(serde_json::json!({
        "ok": true,
        "version": installed.version,
        "restarting": cfg!(unix),
    })))
}
