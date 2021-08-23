use std::sync::Arc;

use super::{cao_sim_model::AxialPos, Connected, NewEntities, NewTerrain};

#[derive(Clone)]
pub struct CaoClient {
    pub runtime: Arc<tokio::runtime::Runtime>,
    pub on_new_entities: (
        crossbeam::channel::Sender<NewEntities>,
        crossbeam::channel::Receiver<NewEntities>,
    ),
    pub send_message: (
        crossbeam::channel::Sender<tungstenite::Message>,
        crossbeam::channel::Receiver<tungstenite::Message>,
    ),
    pub on_new_terrain: (
        crossbeam::channel::Sender<NewTerrain>,
        crossbeam::channel::Receiver<NewTerrain>,
    ),
    pub on_connected: (
        crossbeam::channel::Sender<Connected>,
        crossbeam::channel::Receiver<Connected>,
    ),
}

impl CaoClient {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to init tokio runtime");
        let runtime = Arc::new(runtime);
        let on_new_entities = crossbeam::channel::bounded(4);
        let on_new_terrain = crossbeam::channel::bounded(4);
        let on_reconnect = crossbeam::channel::bounded(2);
        let send_message = crossbeam::channel::bounded(64);
        Self {
            runtime,
            on_new_entities,
            send_message,
            on_new_terrain,
            on_connected: on_reconnect,
        }
    }

    fn send_message(&self, pl: tungstenite::Message) {
        self.send_message.0.send(pl).expect("Failed to send");
    }

    /// subscribes to given room updates
    pub fn send_subscribe_room(&self, room: AxialPos) {
        let payload = serde_json::to_vec(&serde_json::json!({
            "ty": "room_id",
            "room_id": room
        }))
        .unwrap();
        self.send_message(tungstenite::Message::Binary(payload));
    }

    pub fn send_subscribe_multi_room(&self, rooms: &[AxialPos]) {
        let payload = serde_json::to_vec(&serde_json::json!({
            "ty": "room_ids",
            "room_ids": rooms
        }))
        .unwrap();
        self.send_message(tungstenite::Message::Binary(payload));
    }

    /// unsubscribes from given room updates
    pub fn send_unsubscribe_room(&self, room: AxialPos) {
        let payload = serde_json::to_vec(&serde_json::json!({
            "ty": "unsubscribe_room_id",
            "room_id": room
        }))
        .unwrap();
        self.send_message(tungstenite::Message::Binary(payload));
    }

    pub fn send_unsubscribe_all(&self) {
        let payload = serde_json::to_vec(&serde_json::json!({
            "ty": "clear_room_ids"
        }))
        .unwrap();
        self.send_message(tungstenite::Message::Binary(payload));
    }
}
