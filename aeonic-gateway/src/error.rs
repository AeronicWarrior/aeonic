use aeonic_core::error::AeonicError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Wrapper that makes AeonicError into an Axum HTTP response.
pub struct GatewayError(pub AeonicError);

impl From<AeonicError> for GatewayError {
    fn from(e: AeonicError) -> Self {
        Self(e)
    }
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self.0 {
            AeonicError::Auth { .. } => (
                StatusCode::UNAUTHORIZED,
                "auth_error",
                self.0.to_string(),
            ),
            AeonicError::RateLimit { .. } => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limit_exceeded",
                self.0.to_string(),
            ),
            AeonicError::PolicyViolation { .. } => (
                StatusCode::FORBIDDEN,
                "policy_violation",
                self.0.to_string(),
            ),
            AeonicError::NoProvider { .. } => (
                StatusCode::BAD_REQUEST,
                "no_provider",
                self.0.to_string(),
            ),
            AeonicError::Routing(_) => (
                StatusCode::BAD_GATEWAY,
                "routing_error",
                self.0.to_string(),
            ),
            AeonicError::Timeout { .. } => (
                StatusCode::GATEWAY_TIMEOUT,
                "timeout",
                self.0.to_string(),
            ),
            AeonicError::TokenLimit { .. } | AeonicError::ContextTooLarge { .. } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "token_limit",
                self.0.to_string(),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                self.0.to_string(),
            ),
        };

        let body = json!({
            "error": {
                "code": code,
                "message": message,
                "type": "aeonic_error"
            }
        });

        (status, Json(body)).into_response()
    }
}
