use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    engine::EngineState,
    entity::image_vote,
    repository::{RepositoryErr, check_if_img_vote_exists, upsert_img_vote},
    ws::broadcast,
};

#[derive(thiserror::Error, Debug)]
pub enum VoteErr {
    #[error("VoteErr: FromRepository: {0}")]
    FromRepository(#[from] RepositoryErr),
}

#[derive(Deserialize)]
pub struct VoteQuery {
    pub img_id: Uuid,
    pub user_id: Uuid,
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
) -> impl IntoResponse {
    match _vote_inner(q, state).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("{e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn _vote_inner(q: VoteQuery, state: EngineState) -> Result<axum::http::StatusCode, VoteErr> {
    let img_vote_op = check_if_img_vote_exists(&state.db, &q.user_id, &q.img_id).await?;
    let is_new = img_vote_op.is_none();

    upsert_img_vote(
        &state.db,
        img_vote_op,
        q.user_id,
        q.img_id.clone(),
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
