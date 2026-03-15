use axum::extract::{Query, State, WebSocketUpgrade};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    engine::EngineState,
    repository::get_room_id_from_keyword,
    ws::{broadcast, handle_socket},
};

#[derive(Deserialize)]
pub struct WsParams {
    pub room_keyword: String,
    pub user_id: Uuid,
}

#[axum::debug_handler]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(q): Query<WsParams>,
    State(state): State<EngineState>,
) -> axum::http::StatusCode {
    if let Some(id) = get_room_id_from_keyword(&state.db, &q.room_keyword)
        .await
        .unwrap()
    {
        let moved_state = state.clone();
        ws.on_upgrade(move |socket| async move {
            let state = moved_state;
            handle_socket(socket, id, q.user_id, &state.manager).await;
        });
        broadcast(&state.manager, id, crate::ws::ServerEvent::OthersJoin);

        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::BAD_REQUEST
    }
}
