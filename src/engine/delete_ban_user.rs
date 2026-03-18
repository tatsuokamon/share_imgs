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
        EngineState, auth::AuthUser, check_if_he_can_take_action_in_room, generate_user_identifier,
    },
    repository::{RepositoryErr, check_if_ban_tag_exists, check_if_he_is_banned, resolve_ban},
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
    pub room_id: Uuid,
}

async fn _delete_ban_user_inner(
    q: DeleteBanUserQuery,
    state: EngineState,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, DeleteBanUserErr> {
    let mut conn = state.pool.get().await?;
    if !check_if_he_can_take_action_in_room(&state.db, &mut conn, &auth.user_id, &q.room_id).await?
    {
        return Ok(axum::http::StatusCode::FORBIDDEN);
    }

    let user_identifier = generate_user_identifier(&auth.user_id);
    if check_if_he_is_banned(&mut conn, &q.room_id, &user_identifier).await? {
        return Ok(axum::http::StatusCode::BAD_REQUEST);
    }
    resolve_ban(&mut conn, &q.room_id, &user_identifier).await?;

    broadcast(
        &state.manager,
        q.room_id,
        ServerEvent::ResolvedUserBan {
            his_identifier: q.user_identifier,
        },
    );

    Ok(axum::http::StatusCode::OK)
}

pub async fn delete_ban_user(
    Query(q): Query<DeleteBanUserQuery>,
    State(state): State<EngineState>,
    auth: AuthUser,
) -> impl IntoResponse {
    match _delete_ban_user_inner(q, state, auth).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("{e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
