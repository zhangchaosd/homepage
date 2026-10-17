use std::sync::Arc;

use axum::{
    Router,
    extract::{DefaultBodyLimit, FromRef},
    http::{HeaderValue, Request, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use rust_embed::Embed;
use tower_http::{catch_panic::CatchPanicLayer, trace::TraceLayer};

use crate::{
    api,
    config::Config,
    error::AppResult,
    security::{
        auth::{AuthStore, require_auth},
        origin::{PublicOrigin, same_origin_writes},
        passkey::PasskeyAuth,
    },
    update::Updater,
    widgets::Hub,
};

#[derive(Clone)]
pub struct AppState {
    pub auth: AuthStore,
    pub hub: Hub,
    pub origin: PublicOrigin,
    pub updater: Arc<Updater>,
}

impl FromRef<AppState> for AuthStore {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}

impl FromRef<AppState> for PublicOrigin {
    fn from_ref(state: &AppState) -> Self {
        state.origin.clone()
    }
}

#[derive(Embed)]
#[folder = "web/dist/"]
struct WebAssets;

pub fn build_auth(config: &Config) -> AppResult<AuthStore> {
    let passkey = match &config.passkey_db {
        Some(path) => {
            let user_name = config.username.as_deref().unwrap_or("owner");
            let passkey = PasskeyAuth::open(path, &config.public_url, user_name)?;
            let credentials = passkey.credential_count()?;
            tracing::info!(rp_id = passkey.rp_id(), origin = %config.public_url, credentials, "passkey login enabled");
            if credentials == 0 {
                tracing::warn!("the passkey database has no registered passkeys");
            }
            Some(passkey)
        }
        None => None,
    };
    let rp_id = passkey.as_ref().map(|passkey| passkey.rp_id().to_owned());
    if config.password_hash.is_some() && config.username.is_none() {
        tracing::warn!("password login has no username; set auth.username in config.toml");
    }
    AuthStore::new(config.password_hash.clone(), config.secure_cookie)
        .with_trusted_proxies(config.trusted_proxies.clone())
        .with_username(config.username.clone())
        .with_passkey(passkey)
        // Sessions survive restarts until the login settings change.
        .with_session_file(
            &config.state_dir.join("sessions.json"),
            &format!(
                "v1\0{}\0{}\0{}",
                config.username.as_deref().unwrap_or_default(),
                config.password_hash.as_deref().unwrap_or_default(),
                rp_id.as_deref().unwrap_or_default()
            ),
        )
}

pub async fn run(config: Config) -> AppResult<()> {
    let auth = build_auth(&config)?;
    let listen = config.listen;
    let origin = PublicOrigin(config.public_url.clone());
    let trusted = config.trusted_proxies.len();
    let updater = Arc::new(Updater::new(&config.state_dir)?);
    // Scheduled checks only record what is available; installing is always explicit.
    let ticker = updater.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            ticker.tick().await;
        }
    });
    let hub = Hub::new(Arc::new(config))
        .map_err(|error| crate::error::AppError::Internal(format!("http client: {error}")))?;
    hub.start();
    let router = router(AppState {
        auth,
        hub,
        origin,
        updater,
    });
    let listener = tokio::net::TcpListener::bind(listen).await?;
    tracing::info!(%listen, trusted_proxies = trusted, "listening");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

pub fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/api/v1/auth/session", get(api::auth_session))
        .route("/api/v1/auth/logout", post(api::logout))
        .route("/api/v1/dashboard", get(api::dashboard))
        .route("/api/v1/refresh/{widget}", post(api::refresh))
        .route("/api/v1/todos/{id}/done", post(api::complete_todo))
        .route("/icons/{name}", get(api::icon))
        .route("/api/v1/update", get(api::update_status))
        .route("/api/v1/update/settings", patch(api::update_settings))
        .route("/api/v1/update/check", post(api::update_check))
        .route("/api/v1/update/install", post(api::update_install))
        .route_layer(middleware::from_fn_with_state(
            state.auth.clone(),
            require_auth,
        ));

    Router::new()
        .route("/healthz", get(api::health))
        .route("/api/v1/system", get(api::system))
        .route("/api/v1/auth/login", post(api::login))
        .route("/api/v1/auth/passkey/begin", post(api::passkey_begin))
        .route("/api/v1/auth/passkey/finish", post(api::passkey_finish))
        .merge(protected)
        .fallback(static_handler)
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::from_fn_with_state(
            state.origin.clone(),
            same_origin_writes,
        ))
        .layer(middleware::from_fn(security_headers))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// The single-page app. Hashed assets are cached forever; everything else,
/// including `index.html`, is revalidated so new releases show up at once.
async fn static_handler(request: Request<axum::body::Body>) -> Response {
    let path = request.uri().path().trim_start_matches('/');
    if path.starts_with("api/") {
        return (StatusCode::NOT_FOUND, "API route not found").into_response();
    }
    let requested = if path.is_empty() { "index.html" } else { path };
    let (name, asset) = match WebAssets::get(requested) {
        Some(asset) => (requested, asset),
        None => match WebAssets::get("index.html") {
            Some(asset) => ("index.html", asset),
            None => return (StatusCode::NOT_FOUND, "frontend build not found").into_response(),
        },
    };
    let mime = mime_guess::from_path(name).first_or_octet_stream();
    let cache = if name.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        [
            (header::CONTENT_TYPE, mime.as_ref().to_owned()),
            (header::CACHE_CONTROL, cache.to_owned()),
        ],
        asset.data,
    )
        .into_response()
}

async fn security_headers(request: Request<axum::body::Body>, next: middleware::Next) -> Response {
    let api = request.uri().path().starts_with("/api/");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    let defaults = [
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        ("x-frame-options", "DENY"),
        ("cross-origin-opener-policy", "same-origin"),
        (
            "content-security-policy",
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'",
        ),
    ];
    for (name, value) in defaults {
        headers
            .entry(name)
            .or_insert(HeaderValue::from_static(value));
    }
    if api {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to install Ctrl+C handler");
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => tracing::error!(%error, "failed to install SIGTERM handler"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { () = ctrl_c => {}, () = terminate => {} }
}
