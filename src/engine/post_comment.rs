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
    engine::{
        EngineState, auth::AuthUser, check_if_he_can_take_action_in_room, generate_user_identifier,
    },
    repository::{
        self, RepositoryErr, check_if_his_comment_waits_enough, update_post_comment_status,
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
    pub room_id: Uuid,
}

#[derive(Deserialize)]
pub struct PostCommentPayload {
    pub display_name: Option<String>,
    pub content: String,
}

#[axum::debug_handler]
pub async fn post_comment(
    Query(q): Query<PostCommentQuery>,
    State(state): State<EngineState>,
    auth: AuthUser,
    Json(payload): Json<PostCommentPayload>,
) -> impl IntoResponse {
    match _post_comment_inner(payload, q, state, auth).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("{e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn _post_comment_inner(
    payload: PostCommentPayload,
    q: PostCommentQuery,
    state: EngineState,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, PostCommentErr> {
    let mut conn = state.pool.get().await?;
    if !check_if_he_can_take_action_in_room(&state.db, &mut conn, &auth.user_id, &q.room_id).await?
    {
        return Ok(axum::http::StatusCode::FORBIDDEN);
    }

    if !check_if_his_comment_waits_enough(&mut conn, &auth.user_id).await? {
        return Ok(axum::http::StatusCode::TOO_MANY_REQUESTS);
    }
    let content = if payload.content.len() < 141 {
        payload.content.clone()
    } else {
        payload.content.get(0..141).unwrap().to_string()
    };

    update_post_comment_status(&mut conn, &auth.user_id, state.post_comment_timeout).await?;
    let comment_id = repository::post_comment(
        &state.db,
        q.room_id,
        auth.user_id,
        payload.display_name.clone(),
        content.clone(),
    )
    .await?;

    broadcast(
        &state.manager,
        q.room_id,
        crate::ws::ServerEvent::CommentPosted {
            id: comment_id,
            display_name: payload.display_name.unwrap_or("無名".to_string()),
            content: content,
            user_identifier: generate_user_identifier(&auth.user_id),
        },
    );

    Ok(axum::http::StatusCode::OK)
}
