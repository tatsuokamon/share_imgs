use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    engine::{EngineState, get_posted_img::GetPostedImgResult},
    entity::comment,
    repository,
};

#[derive(Serialize)]
pub struct GetPostedComment {
    pub payload: Option<Vec<comment::Model>>,
    pub success: bool,
}

#[derive(Deserialize)]
pub struct GetPostedQuery {
    pub room_id: Uuid,
}

pub async fn get_posted_comment(
    Query(q): Query<GetPostedQuery>,
    State(state): State<EngineState>,
) -> impl IntoResponse {
    match repository::get_posted_comments(&state.db, &q.room_id).await {
        Ok(result) => (
            axum::http::StatusCode::OK,
            Json(GetPostedImgResult {
                payload: Some(result),
                success: true,
            }),
        ),

        Err(e) => {
            tracing::error!("{e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(GetPostedImgResult {
                    payload: None,
                    success: false,
                }),
            )
        }
    }
}
