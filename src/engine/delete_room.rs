use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    engine::{EngineState, auth::AuthUser},
    repository::{self, RepositoryErr, check_if_he_is_authorized},
};

#[derive(thiserror::Error, Debug)]
pub enum DeleteRoomErr {
    #[error("DeleteRoomErr: RepositoryErr: {0}")]
    RepositoryErr(#[from] RepositoryErr),
}

#[derive(Deserialize)]
pub struct DeleteRoomQuery {
    pub room_id: Uuid,
}

pub async fn delete_room(
    Query(q): Query<DeleteRoomQuery>,
    State(state): State<EngineState>,
    auth: AuthUser,
) -> impl IntoResponse {
    match _delete_room_inner(q, state, auth).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("{e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn _delete_room_inner(
    q: DeleteRoomQuery,
    state: EngineState,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, DeleteRoomErr> {
    if !check_if_he_is_authorized(&state.db, &auth.user_id, &q.room_id).await? {
        return Ok(axum::http::StatusCode::FORBIDDEN);
    }
    repository::delete_room(&state.db, &q.room_id).await?;

    Ok(axum::http::StatusCode::OK)
}
