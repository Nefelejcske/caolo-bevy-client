//! Components / Systems related to interacting with the remote Simulation
//!

pub mod cao_sim_model;

use bevy::prelude::*;
use futures::prelude::*;

use std::sync::Arc;

use self::cao_sim_model::AxialPos;

pub struct CaoSimPlugin;

type Ws = async_tungstenite::WebSocketStream<async_tungstenite::tokio::ConnectStream>;

struct NewEntitiesRcv(crossbeam::channel::Receiver<NewEntities>);
struct MessageSender(crossbeam::channel::Sender<Vec<u8>>);

#[derive(Clone)]
pub struct CaoClient {
    pub runtime: Arc<tokio::runtime::Runtime>,
    pub on_new_entities: (
        crossbeam::channel::Sender<NewEntities>,
        crossbeam::channel::Receiver<NewEntities>,
    ),
    pub send_message: (
        crossbeam::channel::Sender<Vec<u8>>,
        crossbeam::channel::Receiver<Vec<u8>>,
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
        let send_message = crossbeam::channel::bounded(32);
        Self {
            runtime,
            on_new_entities,
            send_message,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SimEntityId(pub i64);

pub struct NewEntities(pub Arc<cao_sim_model::EntitiesPayload>);

pub fn hex_axial_to_pixel(q: f32, r: f32) -> Vec2 {
    const SQRT3: f32 = 1.732_050_8;
    const SQRT3_2: f32 = 0.866_025_4;
    const THREE_OVER_TWO: f32 = 1.5;

    Vec2::new(q * SQRT3 + r * SQRT3_2, r * THREE_OVER_TWO)
}

/// Fire NewEntities event and reset current_entities
fn send_new_entities(recv: Res<NewEntitiesRcv>, mut on_new_entities: EventWriter<NewEntities>) {
    while let Ok(entities) = recv.0.recv_timeout(std::time::Duration::from_micros(1)) {
        on_new_entities.send(entities);
    }
}

async fn get_connection() -> Ws {
    let (ws_stream, _resp) =
        async_tungstenite::tokio::connect_async("wss://rt-snorrwe.cloud.okteto.net/object-stream")
            .await
            .expect("Failed to connect to object-stream"); // TODO: handle errors and re-try
    debug!("Successfully connected to object-stream");
    ws_stream
}

// TODO: error pls
async fn send_current_room(stream: &mut Ws, room: AxialPos) {
    let initial_pl = serde_json::to_vec(&serde_json::json!({
        "ty": "room_id",
        "room_id": room
    }))
    .unwrap();
    stream
        .send(tungstenite::Message::Binary(initial_pl))
        .await
        .unwrap();
}

fn setup(client: ResMut<CaoClient>) {
    let _msg_recv = client.send_message.1.clone();
    let entities_sender = client.on_new_entities.0.clone();

    client.runtime.spawn(async move {
        loop {
            info!("Connecting to game-object steam");

            let mut ws_stream = get_connection().await;
            // TODO: current room
            send_current_room(&mut ws_stream, AxialPos { q: 15, r: 15 }).await;

            while let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(tungstenite::Message::Text(txt)) => {
                        let msg = serde_json::from_str::<cao_sim_model::Message>(txt.as_str())
                            .expect("Failed to deserialize msg");
                        match msg {
                            cao_sim_model::Message::Terrain(_terrain) => {
                                info!("Got terrain");
                            }
                            cao_sim_model::Message::Entities(ent) => {
                                debug!("New entities, time: {}", ent.time);
                                entities_sender
                                    .send(NewEntities(Arc::new(ent)))
                                    .expect("Failed to send new entities");
                            }
                        }
                    }
                    Ok(tungstenite::Message::Pong(_)) => {
                        trace!("Server pong received")
                    }
                    Ok(tungstenite::Message::Ping(_)) => {
                        ws_stream
                            .send(tungstenite::Message::Pong(Vec::new()))
                            .await
                            .expect("failed to pong");
                    }
                    Ok(msg) => {
                        debug!("Unexpected message variant {:?}", msg);
                    }
                    Err(err) => {
                        info!("Connection dropped ({}), reconnecting", err);
                    }
                }
            }
        }
    });
}

impl Plugin for CaoSimPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let client = CaoClient::new();
        app.add_startup_system(setup.system())
            .add_event::<NewEntities>()
            .add_system(send_new_entities.system())
            .insert_resource(NewEntitiesRcv(client.on_new_entities.1.clone()))
            .insert_resource(MessageSender(client.send_message.0.clone()))
            .insert_resource(client);
    }
}
