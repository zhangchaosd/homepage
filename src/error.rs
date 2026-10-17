use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use uuid::Uuid;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    InvalidRequest(String),
    #[error("resource not found")]
    NotFound,
    #[error("authentication required")]
    Unauthenticated,
    #[error("request is forbidden")]
    Forbidden,
    #[error("too many authentication attempts; retry after {retry_after_seconds} seconds")]
    RateLimited { retry_after_seconds: u64 },
    #[error("upstream service failed: {0}")]
    Upstream(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("internal server error")]
    Internal(String),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    error: &'static str,
    message: String,
    request_id: Uuid,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, retry_after) = match &self {
            Self::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid_request", None),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", None),
            Self::Unauthenticated => (StatusCode::UNAUTHORIZED, "unauthenticated", None),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden", None),
            Self::RateLimited {
                retry_after_seconds,
            } => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                Some(*retry_after_seconds),
            ),
            Self::Upstream(_) => (StatusCode::BAD_GATEWAY, "upstream_failed", None),
            Self::Io(_) | Self::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", None)
            }
        };
        if status.is_server_error() {
            match &self {
                Self::Internal(detail) => tracing::error!(%detail, "request failed"),
                other => tracing::error!(error = %other, "request failed"),
            }
        }
        let body = ErrorBody {
            error: code,
            message: self.to_string(),
            request_id: Uuid::new_v4(),
        };
        let mut response = (status, Json(body)).into_response();
        if let Some(seconds) = retry_after {
            response.headers_mut().insert(
                header::RETRY_AFTER,
                HeaderValue::from_str(&seconds.to_string())
                    .unwrap_or_else(|_| HeaderValue::from_static("1")),
            );
        }
        response
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::Internal(error.to_string())
    }
}
