//! Components / Systems related to interacting with the remote Simulation
//!

pub mod cao_sim_model;

use bevy::{prelude::*, tasks::IoTaskPool};
use tungstenite::connect;

use std::sync::{Arc, Mutex};

use self::cao_sim_model::AxialPos;

pub struct CaoSimPlugin;

type Ws = tungstenite::WebSocket<
    tungstenite::stream::Stream<std::net::TcpStream, native_tls::TlsStream<std::net::TcpStream>>,
>;

#[derive(Default, Clone)]
pub struct WsConn(pub Option<Arc<Mutex<Ws>>>);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SimEntityId(pub i64);

pub struct NewEntities(pub cao_sim_model::EntitiesPayload);

#[derive(Default)]
pub struct CurrentEntities(pub Arc<Mutex<cao_sim_model::EntitiesPayload>>);

pub fn hex_axial_to_pixel(q: f32, r: f32) -> Vec2 {
    const SQRT3: f32 = 1.7320508075688772935274463415059;
    const SQRT3_2: f32 = 0.86602540378443864676372317075294;
    const THREE_OVER_TWO: f32 = 1.5;

    Vec2::new(q * SQRT3 + r * SQRT3_2, r * THREE_OVER_TWO)
}

/// Fire NewEntities event and reset current_entities
fn send_new_entities(
    mut last_time: Local<i64>, // last seen time
    current_entities: Res<CurrentEntities>,
    mut on_new_entities: EventWriter<NewEntities>,
) {
    let mut current_entities = current_entities.0.lock().unwrap();

    if current_entities.time == *last_time {
        return;
    }

    *last_time = current_entities.time;
    let current: cao_sim_model::EntitiesPayload = std::mem::take(&mut *current_entities);
    current_entities.time = current.time; // current_entities have been replaced with Default, set the time to current so we don't fire the event again
    debug!("Received new state: time: {}", current.time);
    on_new_entities.send(NewEntities(current));
}

fn update_world(conn: Res<WsConn>, pool: Res<IoTaskPool>, current_entities: Res<CurrentEntities>) {
    if let Some(ws_stream) = conn.0.as_ref() {
        let ws_stream = Arc::clone(&ws_stream);
        let current_entities = Arc::clone(&current_entities.0);
        pool.0
            .spawn(async move {
                let mut ws_stream = ws_stream.lock().unwrap();
                if ws_stream.can_write() {
                    match ws_stream.read_message() {
                        Ok(tungstenite::Message::Text(txt)) => {
                            let msg = serde_json::from_str::<cao_sim_model::Message>(txt.as_str())
                                .expect("Failed to deserialize msg");
                            match msg {
                                cao_sim_model::Message::Terrain(_terrain) => {
                                    info!("Got terrain");
                                }
                                cao_sim_model::Message::Entities(ent) => {
                                    debug!("New entities, time: {}", ent.time);
                                    let mut current_entities = current_entities.lock().unwrap();
                                    *current_entities = ent;
                                }
                            }
                        }
                        Ok(tungstenite::Message::Pong(_)) => {
                            trace!("Server pong received")
                        }
                        Ok(tungstenite::Message::Ping(_)) => {
                            ws_stream
                                .write_message(tungstenite::Message::Pong(Vec::new()))
                                .expect("failed to pong");
                        }
                        Ok(msg) => {
                            debug!("Unexpected message variant {:?}", msg);
                        }
                        Err(err) => {
                            info!("Connection dropped ({}), reconnecting", err);
                            *ws_stream = get_connection();
                            send_current_room(&mut ws_stream, AxialPos { q: 15, r: 15 });
                        }
                    }
                } else {
                    info!("Connection dropped, reconnecting");
                    *ws_stream = get_connection();
                    send_current_room(&mut ws_stream, AxialPos { q: 15, r: 15 });
                }
            })
            .detach();
    }
}

fn get_connection() -> Ws {
    let (ws_stream, _resp) = connect("wss://rt-snorrwe.cloud.okteto.net/object-stream")
        .expect("Failed to connect to object-stream"); // TODO
    debug!("Successfully connected to object-stream");
    ws_stream
}

fn send_current_room(ws_stream: &mut Ws, room: AxialPos) {
    let initial_pl = serde_json::to_vec(&serde_json::json!({
        "ty": "room_id",
        "room_id": room
    }))
    .unwrap();

    ws_stream
        .write_message(tungstenite::Message::Binary(initial_pl))
        .unwrap();
}

fn setup(mut conn: ResMut<WsConn>, pool: Res<IoTaskPool>) {
    if conn
        .0
        .as_ref()
        .map(|ws_stream| {
            ws_stream
                .try_lock()
                .map(|ws_stream| ws_stream.can_read() && ws_stream.can_write())
                .unwrap_or(false)
        })
        .unwrap_or(false)
    {
        // connection is valid
        info!("connection is valid");
        return;
    }

    info!("Connecting to game-object steam");

    let ws_stream = get_connection();
    let ws_stream = Arc::new(Mutex::new(ws_stream));
    conn.0 = Some(Arc::clone(&ws_stream));

    pool.0
        .spawn(async move {
            let mut ws_stream = ws_stream.lock().unwrap();
            send_current_room(&mut ws_stream, AxialPos { q: 15, r: 15 });
        })
        .detach();
}

impl Plugin for CaoSimPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_event::<NewEntities>()
            .add_system(update_world.system())
            .add_system(send_new_entities.system())
            .init_resource::<WsConn>()
            .init_resource::<CurrentEntities>();
    }
}
