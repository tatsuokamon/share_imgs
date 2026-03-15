use std::sync::Arc;

use axum::{Router, routing};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use sea_orm::DatabaseConnection;
use sha2::Digest;
use uuid::Uuid;

use crate::{
    engine::{
        delete_comment::delete_comment, delete_img::delete_img,
        get_posted_comment::get_posted_comment, get_posted_img::get_posted_img,
        post_comment::post_comment, post_img::post_img, ws_handler::ws_handler,
    },
    ws::WsManager,
};

mod get_posted_comment;
mod get_posted_img;

mod delete_comment;
mod post_comment;

mod delete_img;
mod get_presigned_url;
mod post_img;

mod vote;
mod ws_handler;

mod delete_ban_user;
mod post_ban_user;

pub struct EngineStateSrc {
    pub db: DatabaseConnection,
    pub sdk_client: aws_sdk_s3::Client,
    pub pool: Pool<RedisConnectionManager>,
    pub manager: WsManager,
    pub bucket_name: String,
    pub expires_in: u64,
    pub post_img_timeout: usize,
    pub post_comment_timeout: u64,
}

pub type EngineState = Arc<EngineStateSrc>;

pub fn generate_user_identifier(user_id: &Uuid) -> String {
    format!("user-{:x}", sha2::Sha256::digest(user_id.as_bytes()))
}

pub async fn generate_router(state: EngineState) -> Router {
    Router::new()
        .route(
            "/get_presigned_url",
            axum::routing::get(get_presigned_url::get_presigned_url),
        )
        .route("/img", axum::routing::post(post_img).delete(delete_img))
        .route("/posted_img", axum::routing::get(get_posted_img))
        .route("/posted_comment", axum::routing::get(get_posted_comment))
        .route(
            "/comment",
            axum::routing::post(post_comment).delete(delete_comment),
        )
        .route("/ws", ws_handler)
        .route("/vote", routing::post(vote::vote))
        // ban
        .with_state(state)
}
