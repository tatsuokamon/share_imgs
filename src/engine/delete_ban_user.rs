use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use bb8::RunError;
use redis::RedisError;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    engine::{
        EngineState, check_if_he_can_take_action_in_room, generate_ban_tag_from_user_identifier,
    },
    repository::{RepositoryErr, check_if_ban_tag_exists, resolve_user_ban_with_tag},
    ws::{ServerEvent, broadcast},
};

#[derive(thiserror::Error, Debug)]
pub enum DeleteBanUserErr {
    #[error("DeleteBanUserErr: RepositoryErr: {0}")]
    RepositoryErr(#[from] RepositoryErr),

    #[error("DeleteBanUserErr: RedisError: {0}")]
    RedisError(#[from] RunError<RedisError>),
}

#[derive(Deserialize)]
pub struct DeleteBanUserQuery {
    pub user_identifier: String,
    pub master_id: Uuid,
    pub room_id: Uuid,
}

async fn _delete_ban_user_inner(
    q: DeleteBanUserQuery,
    state: EngineState,
) -> Result<axum::http::StatusCode, DeleteBanUserErr> {
    let mut conn = state.pool.get().await?;
    if check_if_he_can_take_action_in_room(&state.db, &mut conn, &q.master_id, &q.room_id).await? {
        return Ok(axum::http::StatusCode::FORBIDDEN);
    }

    let ban_tag = generate_ban_tag_from_user_identifier(&q.user_identifier, Some(&q.room_id));
    if !check_if_ban_tag_exists(&mut conn, &ban_tag).await? {
        return Ok(axum::http::StatusCode::BAD_REQUEST);
    }

    resolve_user_ban_with_tag(&mut conn, ban_tag).await?;
    broadcast(
        &state.manager,
        q.room_id.clone(),
        ServerEvent::ResolvedUserBan {
            his_identifier: q.user_identifier,
        },
    );

    Ok(axum::http::StatusCode::OK)
}

pub async fn delete_ban_user(
    Query(q): Query<DeleteBanUserQuery>,
    State(state): State<EngineState>,
) -> impl IntoResponse {
    match _delete_ban_user_inner(q, state).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("{e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
