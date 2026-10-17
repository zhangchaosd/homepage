//! Defence in depth next to SameSite cookies and CSRF tokens: a browser that
//! sends an `Origin` on a state-changing request must send exactly the
//! configured public origin.

use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::error::AppError;

#[derive(Clone)]
pub struct PublicOrigin(pub String);

pub async fn same_origin_writes(
    State(PublicOrigin(expected)): State<PublicOrigin>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let safe = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let allowed = safe
        || match request.headers().get(header::ORIGIN) {
            None => true,
            Some(origin) => origin.to_str().is_ok_and(|origin| origin == expected),
        };
    if allowed {
        next.run(request).await
    } else {
        AppError::Forbidden.into_response()
    }
}
