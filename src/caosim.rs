//! Components / Systems related to interacting with the remote Simulation
//!

mod cao_sim_model;

use bevy::{prelude::*, tasks::IoTaskPool};
use tungstenite::connect;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::bots::{spawn_bot, Bot};

use self::cao_sim_model::AxialPos;

pub struct CaoSimPlugin;

#[derive(Default, Clone)]
pub struct WsConn(
    pub  Option<
        Arc<
            Mutex<
                tungstenite::WebSocket<
                    tungstenite::stream::Stream<
                        std::net::TcpStream,
                        native_tls::TlsStream<std::net::TcpStream>,
                    >,
                >,
            >,
        >,
    >,
);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SimEntityId(pub i64);

#[derive(Default)]
struct EntityMaps {
    caoid2bevy: HashMap<SimEntityId, Entity>,
}

pub struct NewEntities(pub cao_sim_model::EntitiesPayload);

#[derive(Default)]
pub struct CurrentEntities(pub Arc<Mutex<cao_sim_model::EntitiesPayload>>);

pub fn hex_axial_to_pixel(q: f32, r: f32) -> Vec2 {
    const SQRT3: f32 = 1.7320508075688772935274463415059;
    const SQRT3_2: f32 = 0.86602540378443864676372317075294;
    const THREE_OVER_TWO: f32 = 1.5;

    Vec2::new(q * SQRT3 + r * SQRT3_2, r * THREE_OVER_TWO)
}

fn on_new_entities(
    mut cmd: Commands,
    mut map: ResMut<EntityMaps>,
    bot_assets: Res<crate::bots::assets::BotRenderingAssets>,
    mut bot_materials: ResMut<Assets<crate::bots::assets::BotMaterial>>,
    mut new_entities: EventReader<NewEntities>,
    mut bots: Query<(&mut crate::bots::LastPos, &mut crate::bots::NextPos), With<Bot>>,
) {
    for new_entities in new_entities.iter() {
        let len = map.caoid2bevy.len();
        let mut prev = std::mem::replace(&mut map.caoid2bevy, HashMap::with_capacity(len));
        let curr = &mut map.caoid2bevy;
        curr.clear();
        for bot in new_entities.0.bots.iter() {
            let cao_id = SimEntityId(bot.id);
            if let Some(bot_id) = prev.remove(&cao_id) {
                curr.insert(cao_id, bot_id);
                debug!("found entity {:?}", bot.id);
                let (mut last, mut next) =
                    bots.get_mut(bot_id).expect("Failed to get bot components");
                last.0 = next.0;
                next.0 = hex_axial_to_pixel(bot.pos.q as f32, bot.pos.r as f32);
            } else {
                let pos = &bot.pos;
                let new_id = spawn_bot(
                    &mut cmd,
                    hex_axial_to_pixel(pos.q as f32, pos.r as f32),
                    &*bot_assets,
                    &mut *bot_materials,
                );

                curr.insert(cao_id, new_id);
                debug!("new entity {:?}", bot.id);
            }
        }
        // these entities were not sent in the current tick
        for (_, dead_entity) in prev {
            cmd.entity(dead_entity).despawn_recursive();
        }
    }
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
    info!("poggies, time: {}", current.time);
    on_new_entities.send(NewEntities(current));
}

fn update_world(conn: Res<WsConn>, pool: Res<IoTaskPool>, current_entities: Res<CurrentEntities>) {
    if let Some(ws_stream) = conn.0.as_ref() {
        let ws_stream = Arc::clone(&ws_stream);
        let current_entities = Arc::clone(&current_entities.0);
        pool.0
            .spawn(async move {
                let mut ws_stream = ws_stream.lock().unwrap();
                if ws_stream.can_read() {
                    ws_stream
                        .write_message(tungstenite::Message::Ping(Vec::new()))
                        .expect("Failed to ping");
                    match ws_stream.read_message() {
                        Ok(tungstenite::Message::Text(txt)) => {
                            let msg = serde_json::from_str::<cao_sim_model::Message>(txt.as_str())
                                .expect("Failed to deserialize msg");
                            match msg {
                                cao_sim_model::Message::Terrain(terrain) => {
                                    info!("Got terrain");
                                }
                                cao_sim_model::Message::Entities(ent) => {
                                    debug!("New entities, time: {}", ent.time,);
                                    let mut current_entities = current_entities.lock().unwrap();
                                    *current_entities = ent;
                                }
                            }
                        }
                        Ok(tungstenite::Message::Ping(_)) => {
                            ws_stream
                                .write_message(tungstenite::Message::Pong(Vec::new()))
                                .expect("failed to pong");
                        }
                        Ok(msg) => {
                            debug!("Unexpected message variant {:?}", msg);
                        }
                        Err(
                            tungstenite::Error::AlreadyClosed
                            | tungstenite::Error::ConnectionClosed,
                        ) => return,
                        Err(err) => {
                            panic!("Failed to read msg: {}", err);
                        }
                    }
                }
            })
            .detach();
    }
}

fn setup(mut conn: ResMut<WsConn>, pool: Res<IoTaskPool>) {
    let (ws_stream, _resp) = connect("wss://rt-snorrwe.cloud.okteto.net/object-stream")
        .expect("Failed to connect to object-stream"); // TODO

    let ws_stream = Arc::new(Mutex::new(ws_stream));
    conn.0 = Some(Arc::clone(&ws_stream));

    pool.0
        .spawn(async move {
            let mut ws_stream = ws_stream.lock().unwrap();

            let initial_pl = serde_json::to_vec(&serde_json::json!({
                "ty": "room_id",
                "room_id": AxialPos { q: 15, r: 15 }
            }))
            .unwrap();

            ws_stream
                .write_message(tungstenite::Message::Binary(initial_pl))
                .unwrap();
        })
        .detach();
}

impl Plugin for CaoSimPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_event::<NewEntities>()
            .add_system(update_world.system())
            .add_system(send_new_entities.system())
            .add_system(on_new_entities.system())
            .init_resource::<WsConn>()
            .init_resource::<EntityMaps>()
            .init_resource::<CurrentEntities>();
    }
}
