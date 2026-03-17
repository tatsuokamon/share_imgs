use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{engine::EngineState, entity::images, repository};

#[derive(Serialize)]
pub struct GetPostedImgResult {
    pub payload: Option<Vec<images::Model>>,
    pub success: bool,
}

#[derive(Deserialize)]
pub struct GetPostedImgQuery {
    pub room_id: Uuid,
}

pub async fn get_posted_img(
    Query(q): Query<GetPostedImgQuery>,
    State(state): State<EngineState>,
) -> impl IntoResponse {
    match repository::get_posted_imgs(&state.db, &q.room_id).await {
        Ok(v) => (
            axum::http::StatusCode::OK,
            Json(GetPostedImgResult {
                payload: Some(v),
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
