use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use bb8::RunError;
use redis::RedisError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    engine::{
        EngineState,
        auth::{self, AuthUser},
        check_if_he_can_take_action_in_room,
    },
    repository::{RepositoryErr, check_if_img_vote_exists, check_if_room_has_img, upsert_img_vote},
    ws::broadcast,
};

#[derive(thiserror::Error, Debug)]
pub enum VoteErr {
    #[error("VoteErr: FromRepository: {0}")]
    FromRepository(#[from] RepositoryErr),

    #[error("VoteErr: RedisErr: {0}")]
    RedisErr(#[from] RunError<RedisError>),
}

#[derive(Deserialize)]
pub struct VoteQuery {
    pub img_id: Uuid,
    pub is_good: bool,
    pub room_id: Uuid,
}

#[derive(Serialize)]
pub struct VoteResult {
    pub img_id: Uuid,
    pub is_good: bool,
    pub is_new: bool,
}

pub async fn vote(
    Query(q): Query<VoteQuery>,
    State(state): State<EngineState>,
    auth: AuthUser,
) -> impl IntoResponse {
    match _vote_inner(q, state, auth).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("{e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn _vote_inner(
    q: VoteQuery,
    state: EngineState,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, VoteErr> {
    let mut conn = state.pool.get().await?;
    if !check_if_he_can_take_action_in_room(&state.db, &mut conn, &auth.user_id, &q.room_id).await?
    {
        return Ok(axum::http::StatusCode::FORBIDDEN);
    }

    if !check_if_room_has_img(&state.db, &q.img_id, &q.room_id).await? {
        return Ok(axum::http::StatusCode::BAD_REQUEST);
    }

    let img_vote_op = check_if_img_vote_exists(&state.db, &auth.user_id, &q.img_id).await?;
    let is_new = img_vote_op.is_none();

    upsert_img_vote(
        &state.db,
        img_vote_op,
        auth.user_id,
        q.img_id,
        q.is_good.clone(),
    )
    .await?;

    broadcast(
        &state.manager,
        q.room_id,
        crate::ws::ServerEvent::VotedUpdated {
            image_id: q.img_id,
            is_good: q.is_good,
            is_new: is_new,
        },
    );

    Ok(axum::http::StatusCode::OK)
}
