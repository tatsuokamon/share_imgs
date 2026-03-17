use aws_sdk_s3::types::error::builders::TooManyPartsBuilder;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::repository::{
    RepositoryErr, check_if_he_exists, check_if_his_img_waits_enough, generate_object_key,
    generate_presigned_url, generate_user, update_commit_img_status,
};

#[derive(Serialize, Clone)]
pub enum ServerEvent {
    ProvidePresignedURL {
        url: Option<String>,
    },

    ImagePosted {
        id: Uuid,
        url: String,
        title: Option<String>,
        display_name: String,
        user_identifier: String,
    },

    ImageDeleted {
        id: Uuid,
    },

    CommentPosted {
        id: Uuid,
        display_name: String,
        content: String,
        user_identifier: String,
    },

    CommentDeleted {
        id: Uuid,
    },

    VotedUpdated {
        image_id: Uuid,
        is_good: bool,
        is_new: bool,
    },

    UserBanned {
        his_identifier: String,
    },

    ResolvedUserBan {
        his_identifier: String,
    },

    RoomDeleted,

    OthersJoin,
    OthersDrop,
}

type Tx = mpsc::UnboundedSender<ServerEvent>;

pub struct WsManager {
    pub rooms: DashMap<Uuid, DashMap<Uuid, Tx>>,
}

pub fn join_room(manager: &WsManager, room_id: Uuid, client_id: Uuid, tx: Tx) {
    manager
        .rooms
        .entry(room_id)
        .or_insert(DashMap::new())
        .insert(client_id, tx);
}

pub fn leave_room(manager: &WsManager, room_id: Uuid, client_id: Uuid) {
    if let Some(room) = manager.rooms.get(&room_id) {
        room.remove(&client_id);
    }

    broadcast(manager, room_id, ServerEvent::OthersDrop);
}

pub fn broadcast(manager: &WsManager, room_id: Uuid, event: ServerEvent) {
    if let Some(room) = manager.rooms.get(&room_id) {
        for entry in room.iter() {
            let _ = entry.value().send(event.clone());
        }
    }
}

pub async fn handle_socket(socket: WebSocket, room_id: Uuid, client_id: Uuid, manager: &WsManager) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    join_room(manager, room_id.clone(), client_id.clone(), tx);
    let (mut sender, mut receiver) = socket.split();

    let send_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let json = serde_json::to_string(&event).unwrap();

            if let Err(e) = sender
                .send(axum::extract::ws::Message::Text(json.into()))
                .await
            {
                tracing::error!("{e}");
            };
        }
    });

    let recv_task = tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    leave_room(manager, room_id, client_id);
}
