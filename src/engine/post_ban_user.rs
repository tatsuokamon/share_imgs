use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use bb8::RunError;
use redis::RedisError;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    engine::{
        EngineState, check_if_he_can_take_action_in_room, generate_ban_tag_from_user_identifier,
        generate_user_identifier,
    },
    repository::{
        self, RepositoryErr, ban_user_with_tag, check_if_he_is_authorized, find_user_id_with_img_id,
    },
    ws::broadcast,
};

#[derive(thiserror::Error, Debug)]
pub enum PostBanUserErr {
    #[error("PostBanUserErr: RepositoryErr: {0}")]
    RepositoryErr(#[from] RepositoryErr),

    #[error("PostBanUserErr: RedisErr: {0}")]
    RedisErr(#[from] RunError<RedisError>),
}

#[derive(Deserialize)]
pub struct BanUserQuery {
    pub room_id: Uuid,
    pub master_id: Uuid,
    pub user_identifier: String,
}

async fn _post_ban_user_inner(
    q: BanUserQuery,
    state: EngineState,
) -> Result<axum::http::StatusCode, PostBanUserErr> {
    let mut conn = state.pool.get().await?;

    if !check_if_he_can_take_action_in_room(&state.db, &mut conn, &q.master_id, &q.room_id).await? {
        return Ok(axum::http::StatusCode::FORBIDDEN);
    }

    if !check_if_he_is_authorized(&state.db, &q.master_id, &q.room_id).await? {
        return Ok(axum::http::StatusCode::FORBIDDEN);
    }

    let ban_tag = generate_ban_tag_from_user_identifier(&q.user_identifier, Some(&q.room_id));
    ban_user_with_tag(&mut conn, state.ban_timeout, ban_tag).await?;

    broadcast(
        &state.manager,
        q.room_id,
        crate::ws::ServerEvent::UserBanned {
            his_identifier: q.user_identifier,
        },
    );

    Ok(axum::http::StatusCode::OK)
}

pub async fn post_ban_user(
    Query(q): Query<BanUserQuery>,
    State(state): State<EngineState>,
) -> impl IntoResponse {
    match _post_ban_user_inner(q, state).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("{e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
