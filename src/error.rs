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
    #[error("Wrong credentials")]
    WrongCredentials,
    #[error("Missing credentials")]
    MissingCredentials,
    #[error("Token creation error")]
    TokenCreation,
    #[error("Invalid token")]
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, self.to_string()),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
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
    #[error("Quota exceeded")]
    QuotaExceeded,
    #[error("Create instance failed")]
    CreateFailed,
    #[error("Delete instance failed")]
    DeleteFailed,
}

impl IntoResponse for InstanceError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            InstanceError::InvalidArgs(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            InstanceError::AlreadyExists => (StatusCode::CONFLICT, self.to_string()),
            InstanceError::QuotaExceeded => (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()),
            InstanceError::CreateFailed => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            InstanceError::DeleteFailed => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
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
