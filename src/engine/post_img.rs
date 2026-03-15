use axum::{
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
        RepositoryErr, commit_img, generate_object_key, get_object_key, update_commit_img_status,
    },
    ws::broadcast,
};

#[derive(thiserror::Error, Debug)]
pub enum PostImgErr {
    #[error("PostImgErr: FromRepository: {0}")]
    FromRepository(#[from] RepositoryErr),

    #[error("PostImgErr: FromRedis: {0}")]
    FromRedis(#[from] RunError<RedisError>),
}

#[derive(Deserialize)]
pub struct PostImgQuery {
    pub user_id: Uuid,
    pub room_id: Uuid,
    pub title: Option<String>,
    pub presigned_url: String,
    pub display_name: Option<String>,
}

pub async fn post_img(
    Query(q): Query<PostImgQuery>,
    State(state): State<EngineState>,
) -> impl IntoResponse {
    match _post_img_inner(q, state).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("{e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn _post_img_inner(
    q: PostImgQuery,
    state: EngineState,
) -> Result<axum::http::StatusCode, PostImgErr> {
    let mut conn = state.pool.get().await?;
    let obj_key = get_object_key(&mut conn, &q.user_id, &q.presigned_url).await?;

    if obj_key.is_none() {
        return Ok(axum::http::StatusCode::BAD_REQUEST);
    }

    let unwrapped_key = obj_key.unwrap();
    let img_id = commit_img(
        &state.db,
        q.room_id.clone(),
        q.user_id.clone(),
        q.title.clone(),
        unwrapped_key.clone(),
    )
    .await?;
    update_commit_img_status(&mut conn, &q.user_id, state.post_img_timeout).await?;

    broadcast(
        &state.manager,
        q.room_id,
        crate::ws::ServerEvent::ImagePosted {
            id: img_id,
            url: unwrapped_key,
            title: q.title,
            display_name: q.display_name.unwrap_or("無名".to_string()),
            user_identifier: generate_user_identifier(&q.user_id),
        },
    );

    Ok(axum::http::StatusCode::OK)
}
