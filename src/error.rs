use std::borrow::Cow;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use tower::BoxError;

pub type Result<T> = std::result::Result<T, BoxError>;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Unauthorized user")]
    UnauthorizedUser,
    #[error("Invalid token")]
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::UnauthorizedUser => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, self.to_string()),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

#[derive(Debug, Error)]
crate enum InstanceError {
    #[error("Invalid arg `{0}`")]
    InvalidArgs(String),
    #[error("Instance already exists")]
    AlreadyExists,
    #[error("Instance is already deleted")]
    AlreadyDeleted,
    #[error("Instance is not yet stoppped")]
    NotYetStopped,
    #[error("{resource} quota exceeded, quota: {quota:?}{unit}, remaining: {remaining:?}{unit}, requested: {requested:?}{unit}")]
    QuotaExceeded {
        resource: String,
        quota: usize,
        remaining: usize,
        requested: usize,
        unit: String,
    },
    #[error("Create instance failed")]
    CreateFailed,
    #[error("Delete instance failed")]
    DeleteFailed,
    #[error("Update instance failed")]
    UpdateFailed,
    #[error("Start instance failed")]
    StartFailed,
    #[error("Stop instance failed")]
    StopFailed,
    #[error("Unsupported image")]
    UnsupportedImage,
    #[error("Unsupported runtime")]
    UnsupportedRuntime,
}

impl IntoResponse for InstanceError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            InstanceError::InvalidArgs(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            InstanceError::AlreadyExists => (StatusCode::CONFLICT, self.to_string()),
            InstanceError::AlreadyDeleted | InstanceError::NotYetStopped => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            InstanceError::QuotaExceeded { .. } => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
            }
            InstanceError::CreateFailed
            | InstanceError::DeleteFailed
            | InstanceError::UpdateFailed
            | InstanceError::StartFailed
            | InstanceError::StopFailed => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            InstanceError::UnsupportedImage | InstanceError::UnsupportedRuntime => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

pub async fn handle_error(error: BoxError) -> impl IntoResponse {
    if error.is::<tower::timeout::error::Elapsed>() {
        return (StatusCode::REQUEST_TIMEOUT, Cow::from("request timed out"));
    }

    if error.is::<tower::load_shed::error::Overloaded>() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Cow::from("service is overloaded, try again later"),
        );
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Cow::from(format!("Unhandled internal error: {}", error)),
    )
}
