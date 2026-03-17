use axum::{
    extract::{Query, State, WebSocketUpgrade},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    engine::EngineState,
    ws::{broadcast, handle_socket},
};

#[derive(Deserialize)]
pub struct WsParams {
    pub room_id: Uuid,
    pub user_id: Uuid,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(q): Query<WsParams>,
    State(state): State<EngineState>,
) -> impl IntoResponse {
    let moved_state = state.clone();
    let _ = ws.on_upgrade(move |socket| async move {
        let state = moved_state;
        handle_socket(socket, q.room_id, q.user_id, &state.manager).await;
    });
}
