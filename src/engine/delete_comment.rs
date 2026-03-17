use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use bb8::RunError;
use redis::RedisError;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    engine::{EngineState, check_if_he_can_take_action_in_room},
    repository::{
        self, RepositoryErr, check_if_comment_exists, check_if_he_exists,
        check_if_he_is_authorized, check_if_room_has_comment,
    },
    ws::broadcast,
};

#[derive(thiserror::Error, Debug)]
pub enum DeleteCommentErr {
    #[error("DeleteCommentErr: FromRepository; {0}")]
    FromRepository(#[from] RepositoryErr),

    #[error("DeleteCommentErr: FromRepository; {0}")]
    FromRedisErr(#[from] RunError<RedisError>),
}

#[derive(Deserialize)]
pub struct DeleteCommentQuery {
    pub master_id: Uuid,
    pub comment_id: Uuid,
    pub room_id: Uuid,
}

pub async fn delete_comment(
    Query(q): Query<DeleteCommentQuery>,
    State(state): State<EngineState>,
) -> impl IntoResponse {
    match _delete_comment_inner(q, state).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("{e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn _delete_comment_inner(
    q: DeleteCommentQuery,
    state: EngineState,
) -> Result<axum::http::StatusCode, DeleteCommentErr> {
    let mut conn = state.pool.get().await?;

    if !check_if_he_can_take_action_in_room(&state.db, &mut conn, &q.master_id, &q.room_id).await? {
        return Ok(axum::http::StatusCode::FORBIDDEN);
    }

    if !check_if_comment_exists(&state.db, &q.comment_id).await? {
        return Ok(axum::http::StatusCode::BAD_REQUEST);
    }

    if !check_if_room_has_comment(&state.db, &q.comment_id, &q.room_id).await? {
        return Ok(axum::http::StatusCode::BAD_REQUEST);
    }

    if !check_if_he_is_authorized(&state.db, &q.master_id, &q.room_id).await? {
        return Ok(axum::http::StatusCode::FORBIDDEN);
    }

    repository::delete_comment(&state.db, &q.comment_id).await?;

    broadcast(
        &state.manager,
        q.room_id,
        crate::ws::ServerEvent::CommentDeleted { id: q.comment_id },
    );
    Ok(axum::http::StatusCode::OK)
}
