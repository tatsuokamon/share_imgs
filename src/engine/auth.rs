use axum::{extract::FromRequestParts, http::StatusCode};
use sea_orm::prelude::async_trait::async_trait;
use uuid::Uuid;

use crate::engine::{EngineState, gen_token::verify_token};

pub struct AuthUser {
    pub user_id: Uuid,
}

impl FromRequestParts<EngineState> for AuthUser {
    type Rejection = StatusCode;
    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &EngineState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let headers = &parts.headers;
            let auth = headers
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .ok_or(StatusCode::UNAUTHORIZED)?;

            let token = auth
                .strip_prefix("Bearer ")
                .ok_or(StatusCode::UNAUTHORIZED)?;

            if let Some(user_id) = verify_token(token, &state.secret) {
                Ok(AuthUser { user_id })
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    }
}
