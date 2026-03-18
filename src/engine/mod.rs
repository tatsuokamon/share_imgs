use std::sync::Arc;

use axum::{Router, middleware, routing};
use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use sha2::Digest;
use uuid::Uuid;

use crate::{
    engine::{
        delete_comment::delete_comment, delete_img::delete_img,
        get_posted_comment::get_posted_comment, get_posted_img::get_posted_img,
        post_comment::post_comment, post_img::post_img, rate_limit::rate_limit_middleware,
        ws_handler::ws_handler,
    },
    repository::{RepositoryErr, check_if_he_exists, check_if_he_is_banned, check_if_room_exists},
    ws::WsManager,
};

mod auth;
mod rate_limit;

mod gen_token;

mod get_user_id;

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
mod get_ban_user;
mod post_ban_user;

mod delete_room;
mod get_room;
mod post_room;

pub struct EngineStateSrc {
    pub db: DatabaseConnection,
    pub sdk_client: aws_sdk_s3::Client,
    pub pool: Pool<RedisConnectionManager>,
    pub manager: WsManager,
    pub bucket_name: String,
    pub expires_in: u64,
    pub post_img_timeout: usize,
    pub post_comment_timeout: usize,
    pub secret: Vec<u8>,
    pub req_per_minute: usize,
}

pub type EngineState = Arc<EngineStateSrc>;

fn generate_user_identifier(user_id: &Uuid) -> String {
    format!("user-{:x}", sha2::Sha256::digest(user_id.as_bytes()))
}

pub fn generate_router(state: EngineState) -> Router {
    Router::new()
        // about user
        .route("/new-user-id", axum::routing::get(get_user_id::get_user_id))
        .route(
            "/ban",
            routing::post(post_ban_user::post_ban_user)
                .delete(delete_ban_user::delete_ban_user)
                .get(get_ban_user::get_ban_user),
        )
        // about room
        .route(
            "/room",
            axum::routing::get(get_room::get_room)
                .delete(delete_room::delete_room)
                .post(post_room::post_room),
        )
        // about img
        .route(
            "/get_presigned_url",
            axum::routing::get(get_presigned_url::get_presigned_url),
        )
        .route("/img", axum::routing::post(post_img).delete(delete_img))
        .route("/posted_img", axum::routing::get(get_posted_img))
        .route("/vote", routing::post(vote::vote))
        // about comment
        .route(
            "/comment",
            axum::routing::post(post_comment).delete(delete_comment),
        )
        .route("/posted_comment", axum::routing::get(get_posted_comment))
        // about ws
        .route("/ws", axum::routing::get(ws_handler))
        .layer(middleware::from_fn(rate_limit_middleware))
        .with_state(state)
}

pub async fn check_if_he_can_take_action_in_room(
    db: &impl ConnectionTrait,
    conn: &mut PooledConnection<'_, RedisConnectionManager>,
    user_id: &Uuid,
    room_id: &Uuid,
) -> Result<bool, RepositoryErr> {
    let user_identifier = generate_user_identifier(user_id);

    Ok(check_if_he_exists(db, user_id).await?
        && check_if_room_exists(db, room_id).await?
        && check_if_he_is_banned(conn, room_id, &user_identifier).await?)
}
