use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use bb8::RunError;
use redis::RedisError;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    engine::{EngineState, auth::AuthUser, check_if_he_can_take_action_in_room},
    repository::{
        self, RepositoryErr, check_if_he_is_authorized, check_if_img_exists, check_if_room_has_img,
    },
    ws::broadcast,
};

#[derive(thiserror::Error, Debug)]
pub enum DeleteImgErr {
    #[error("DeleteImgErr: FromRepository; {0}")]
    FromRepository(#[from] RepositoryErr),

    #[error("DeleteImgErr: FromRepository; {0}")]
    FromRedisErr(#[from] RunError<RedisError>),
}

#[derive(Deserialize)]
pub struct DeleteImgQuery {
    pub img_id: Uuid,
    pub room_id: Uuid,
}

pub async fn delete_img(
    Query(q): Query<DeleteImgQuery>,
    State(state): State<EngineState>,
    auth: AuthUser,
) -> impl IntoResponse {
    match _delete_img_inner(q, state, auth).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("{e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn _delete_img_inner(
    q: DeleteImgQuery,
    state: EngineState,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, DeleteImgErr> {
    let mut conn = state.pool.get().await?;
    if !check_if_he_can_take_action_in_room(&state.db, &mut conn, &auth.user_id, &q.room_id).await?
    {
        return Ok(axum::http::StatusCode::FORBIDDEN);
    }

    if !check_if_img_exists(&state.db, &q.img_id).await? {
        return Ok(axum::http::StatusCode::BAD_REQUEST);
    }

    if !check_if_room_has_img(&state.db, &q.img_id, &q.room_id).await? {
        return Ok(axum::http::StatusCode::BAD_REQUEST);
    }

    if !check_if_he_is_authorized(&state.db, &auth.user_id, &q.room_id).await? {
        return Ok(axum::http::StatusCode::FORBIDDEN);
    }
    repository::delete_img(&state.db, q.img_id).await?;

    broadcast(
        &state.manager,
        q.room_id,
        crate::ws::ServerEvent::ImageDeleted { id: q.img_id },
    );

    Ok(axum::http::StatusCode::OK)
}
