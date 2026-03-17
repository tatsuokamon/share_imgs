use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    engine::{EngineState, auth::AuthUser},
    repository::{RepositoryErr, generate_room},
};

#[derive(thiserror::Error, Debug)]
pub enum PostRoomErr {
    #[error("PostRoomErr: FromRepositoryErr: {0}")]
    FromRepositoryErr(#[from] RepositoryErr),
}

#[derive(Deserialize)]
pub struct PostRoomQuery {
    pub keyword: String,
}

async fn _post_room_inner(
    q: PostRoomQuery,
    state: EngineState,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, PostRoomErr> {
    generate_room(&state.db, q.keyword, auth.user_id).await?;

    Ok(axum::http::StatusCode::OK)
}

pub async fn post_room(
    Query(q): Query<PostRoomQuery>,
    State(state): State<EngineState>,
    auth: AuthUser,
) -> impl IntoResponse {
    match _post_room_inner(q, state, auth).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("{e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
