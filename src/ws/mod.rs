use axum::extract::ws::WebSocket;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::repository::{
    check_if_he_exists, check_if_his_img_waits_enough, generate_object_key, generate_presigned_url,
    update_commit_img_status,
};

#[derive(Serialize, Clone)]
pub enum ServerEvent {
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

#[derive(Default)]
pub struct WsManager {
    pub rooms: DashMap<Uuid, DashMap<Uuid, Tx>>,
}

impl WsManager {
    pub fn new() -> Self {
        WsManager::default()
    }
}

pub fn join_room(manager: &WsManager, room_id: Uuid, client_id: Uuid, tx: Tx) {
    manager
        .rooms
        .entry(room_id)
        .or_insert(DashMap::new())
        .insert(client_id, tx);
}

pub fn leave_room(manager: &WsManager, room_id: Uuid, client_id: &Uuid) {
    if let Some(room) = manager.rooms.get(&room_id) {
        room.remove(&client_id);

        if room.is_empty() {
            manager.rooms.remove(&room_id);
        }
    }

    broadcast(manager, room_id, ServerEvent::OthersDrop);
}

pub fn broadcast(manager: &WsManager, room_id: Uuid, event: ServerEvent) {
    let mut to_remove = vec![];
    if let Some(room) = manager.rooms.get(&room_id) {
        for entry in room.iter() {
            if let Err(e) = entry.value().send(event.clone()) {
                tracing::error!("{e}");
                to_remove.push(*entry.key())
            };
        }
    }

    for id in to_remove {
        leave_room(manager, room_id, &id);
    }
}

pub async fn handle_socket(socket: WebSocket, room_id: Uuid, client_id: Uuid, manager: &WsManager) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    join_room(manager, room_id, client_id, tx);
    let (mut sender, mut receiver) = socket.split();

    broadcast(manager, room_id, ServerEvent::OthersJoin);
    let send_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let json;
            match serde_json::to_string(&event) {
                Ok(ok) => {
                    json = ok;
                }
                Err(e) => {
                    tracing::error!("{e}");
                    continue;
                }
            };

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

    leave_room(manager, room_id, &client_id);
}
