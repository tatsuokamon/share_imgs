use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use bb8::RunError;
use redis::RedisError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    engine::EngineState,
    repository::{RepositoryErr, get_all_banned_users},
};

#[derive(thiserror::Error, Debug)]
pub enum GetBanUserErr {
    #[error("GetBanUserErr: FromRepository: {0}")]
    RepositoryErr(#[from] RepositoryErr),

    #[error("GetBanUserErr: RedisError: {0}")]
    RedisError(#[from] RunError<RedisError>),
}

#[derive(Deserialize)]
pub struct GetBanUserQuery {
    pub room_id: Uuid,
}

#[derive(Serialize)]
pub struct GetBanUserResult {
    pub success: bool,
    pub banned_users: Option<Vec<String>>,
}

async fn _get_ban_user_inner(
    q: GetBanUserQuery,
    state: EngineState,
) -> Result<(axum::http::StatusCode, Json<GetBanUserResult>), GetBanUserErr> {
    let mut conn = state.pool.get().await?;
    Ok((
        axum::http::StatusCode::OK,
        Json(GetBanUserResult {
            banned_users: get_all_banned_users(&mut conn, q.room_id).await?,
            success: true,
        }),
    ))
}

pub async fn get_ban_user(
    Query(q): Query<GetBanUserQuery>,
    State(state): State<EngineState>,
) -> impl IntoResponse {
    match _get_ban_user_inner(q, state).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("{e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(GetBanUserResult {
                    success: false,
                    banned_users: None,
                }),
            )
        }
    }
}
