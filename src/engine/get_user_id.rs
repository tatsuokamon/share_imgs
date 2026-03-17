use axum::{Json, extract::State, response::IntoResponse};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    engine::{EngineState, generate_user_identifier},
    repository::{RepositoryErr, generate_user},
};

#[derive(Serialize)]
pub struct GetUserIdStruct {
    pub user_id: Option<Uuid>,
    pub user_identifier: Option<String>,
    pub success: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum GetUserIdErr {
    #[error("GetUserIdErr: RepositoryErr: {0}")]
    RepositoryErr(#[from] RepositoryErr),
}

async fn _get_user_id_inner(state: EngineState) -> Result<GetUserIdStruct, GetUserIdErr> {
    let user_id = generate_user(&state.db).await?;
    let user_identifier = generate_user_identifier(&user_id);

    Ok(GetUserIdStruct {
        user_id: Some(user_id),
        user_identifier: Some(user_identifier),
        success: true,
    })
}

pub async fn get_user_id(State(state): State<EngineState>) -> impl IntoResponse {
    match _get_user_id_inner(state).await {
        Ok(res) => (axum::http::StatusCode::OK, Json(res)),
        Err(e) => {
            tracing::error!("{e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(GetUserIdStruct {
                    user_identifier: None,
                    user_id: None,
                    success: false,
                }),
            )
        }
    }
}
