use serde::Deserialize;
use uuid::Uuid;

use crate::repository::RepositoryErr;

#[derive(thiserror::Error, Debug)]
pub enum PostBanUserErr {
    #[error("PostBanUserErr: RepositoryErr: {0}")]
    RepositoryErr(#[from] RepositoryErr),
}

#[derive(Deserialize)]
pub struct BanUserQuery {
    pub room_id: Uuid,
    pub img_id: Uuid,
}

pub async fn _post_ban_user_inner() -> Result<axum::http::StatusCode, PostBanUserErr> {}
