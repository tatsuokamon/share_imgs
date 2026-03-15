use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use bb8::RunError;
use redis::RedisError;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    engine::{EngineState, generate_user_identifier},
    repository::{
        self, RepositoryErr, check_if_he_take_action_in_room, check_if_his_comment_waits_enough,
    },
    ws::broadcast,
};

#[derive(thiserror::Error, Debug)]
pub enum PostCommentErr {
    #[error("PostCommentErr: FromRepository: {0}")]
    FromRepository(#[from] RepositoryErr),

    #[error("PostCommentErr: RedisErr: {0}")]
    RedisErr(#[from] RunError<RedisError>),
}

#[derive(Deserialize)]
pub struct PostCommentQuery {
    pub user_id: Uuid,
    pub room_id: Uuid,
}

#[derive(Deserialize)]
pub struct PostCommentPayload {
    pub display_name: Option<String>,
    pub content: String,
}

pub async fn post_comment(
    q: Query<PostCommentQuery>,
    state: State<EngineState>,
    payload: Json<PostCommentPayload>,
) -> impl IntoResponse {
    match _post_comment_inner(payload, q, state).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("{e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn _post_comment_inner(
    Json(payload): Json<PostCommentPayload>,
    Query(q): Query<PostCommentQuery>,
    State(state): State<EngineState>,
) -> Result<axum::http::StatusCode, PostCommentErr> {
    let mut conn = state.pool.get().await?;
    if !check_if_he_take_action_in_room(&state.db, &mut conn, &q.user_id, &q.room_id).await? {
        return Ok(axum::http::StatusCode::FORBIDDEN);
    }

    if !check_if_his_comment_waits_enough(&mut conn, &q.user_id).await? {
        return Ok(axum::http::StatusCode::TOO_MANY_REQUESTS);
    }

    let comment_id = repository::post_comment(
        &state.db,
        q.room_id.clone(),
        q.user_id.clone(),
        payload.display_name.clone(),
        payload.content.clone(),
    )
    .await?;

    broadcast(
        &state.manager,
        q.room_id,
        crate::ws::ServerEvent::CommentPosted {
            id: comment_id,
            display_name: payload.display_name.unwrap_or("無名".to_string()),
            content: payload.content,
            user_identifier: generate_user_identifier(&q.user_id),
        },
    );

    Ok(axum::http::StatusCode::OK)
}
