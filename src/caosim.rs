//! Components / Systems related to interacting with the remote Simulation
//!

pub mod cao_sim_model;
pub mod terrain_model;

use bevy::prelude::*;
use futures::prelude::*;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use self::cao_sim_model::{AxialPos, RoomId, TerrainTy};

pub struct CaoSimPlugin;
pub struct NewEntities(pub Arc<cao_sim_model::EntitiesPayload>);
pub struct NewTerrain {
    pub room_id: RoomId,
    pub terrain: Arc<Vec<(AxialPos, TerrainTy)>>,
}
pub struct Connected;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(usize)]
pub enum ConnectionState {
    Connecting = 0,
    Online = 1,
    Closed = 2,
    Error = 3,
}

#[derive(Debug, Clone)]
pub struct ConnectionStateRes(Arc<AtomicUsize>);

impl ConnectionStateRes {
    fn new(state: ConnectionState) -> Self {
        Self(Arc::new(AtomicUsize::new(state as usize)))
    }

    fn store(&self, state: ConnectionState, ord: Ordering) {
        self.0.store(state as usize, ord);
    }

    pub fn load(&self, ord: Ordering) -> ConnectionState {
        let value: usize = self.0.load(ord);

        match value {
            0 => ConnectionState::Connecting,
            1 => ConnectionState::Online,
            2 => ConnectionState::Closed,
            3 => ConnectionState::Error,
            _ => unreachable!(),
        }
    }
}

type Ws = async_tungstenite::WebSocketStream<async_tungstenite::tokio::ConnectStream>;

struct NewEntitiesRcv(crossbeam::channel::Receiver<NewEntities>);
struct NewTerrainRcv(crossbeam::channel::Receiver<NewTerrain>);
struct ConnectedRcv(crossbeam::channel::Receiver<Connected>);
struct MessageSender(crossbeam::channel::Sender<tungstenite::Message>);

#[derive(Clone)]
pub struct CaoClient {
    pub runtime: Arc<tokio::runtime::Runtime>,
    on_new_entities: (
        crossbeam::channel::Sender<NewEntities>,
        crossbeam::channel::Receiver<NewEntities>,
    ),
    send_message: (
        crossbeam::channel::Sender<tungstenite::Message>,
        crossbeam::channel::Receiver<tungstenite::Message>,
    ),
    on_new_terrain: (
        crossbeam::channel::Sender<NewTerrain>,
        crossbeam::channel::Receiver<NewTerrain>,
    ),
    on_connected: (
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

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SimEntityId(pub i64);

pub fn hex_axial_to_pixel(q: f32, r: f32) -> Vec2 {
    const SQRT3: f32 = 1.732_050_8;
    const SQRT3_2: f32 = 0.866_025_4;
    const THREE_OVER_TWO: f32 = 1.5;

    Vec2::new(q * SQRT3 + r * SQRT3_2, r * THREE_OVER_TWO)
}

fn send_new_terrain_system(recv: Res<NewTerrainRcv>, mut on_new_terrain: EventWriter<NewTerrain>) {
    while let Ok(terrain) = recv.0.recv_timeout(Duration::from_micros(1)) {
        on_new_terrain.send(terrain);
    }
}

fn send_connected_event_system(recv: Res<ConnectedRcv>, mut on_reconnect: EventWriter<Connected>) {
    while let Ok(pl) = recv.0.recv_timeout(Duration::from_micros(1)) {
        on_reconnect.send(pl);
    }
}

/// Fire NewEntities event and reset current_entities
fn send_new_entities_system(
    recv: Res<NewEntitiesRcv>,
    mut on_new_entities: EventWriter<NewEntities>,
) {
    while let Ok(entities) = recv.0.recv_timeout(Duration::from_micros(1)) {
        on_new_entities.send(entities);
    }
}

async fn get_connection() -> Result<Ws, tungstenite::error::Error> {
    async_tungstenite::tokio::connect_async("wss://rt-snorrwe.cloud.okteto.net/object-stream")
        .await
        .map(|(stream, _resp)| {
            debug!("Successfully connected to object-stream");
            stream
        })
        .map_err(|err| {
            error!("Failed to connect to object-stream {:?}", err);
            err
        })
}

async fn msg_sender<S>(msg_recv: crossbeam::channel::Receiver<tungstenite::Message>, mut tx: S)
where
    S: Sink<tungstenite::Message> + std::marker::Unpin,
    <S as Sink<tungstenite::Message>>::Error: std::fmt::Debug,
{
    loop {
        match msg_recv.try_recv() {
            Ok(msg) => {
                if let Err(err) = tx.send(msg).await {
                    debug!("Send failed {:?}", err);
                    break;
                }
            }
            Err(crossbeam::channel::TryRecvError::Disconnected) => break,
            Err(crossbeam::channel::TryRecvError::Empty) => {
                // reduce cpu pressure
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
        tokio::task::yield_now().await;
    }
}

fn setup(client: Res<CaoClient>, state: Res<ConnectionStateRes>) {
    let msg_send = client.send_message.0.clone();
    let msg_recv = client.send_message.1.clone();
    let entities_sender = client.on_new_entities.0.clone();
    let terrain_sender = client.on_new_terrain.0.clone();
    let reconnect_sender = client.on_connected.0.clone();

    let state: ConnectionStateRes = state.clone();
    let runtime = client.runtime.clone();
    client.runtime.spawn(async move {
        let mut backoff = 1;
        loop {
            info!("Connecting to caosim stream");

            state.store(ConnectionState::Connecting, Ordering::Release);

            let ws_stream;
            match get_connection().await {
                Ok(s) => {
                    backoff = 1;
                    ws_stream = s;
                }
                Err(_) => {
                    debug!("Retrying");
                    state.store(ConnectionState::Error, Ordering::Release);

                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                    backoff = (backoff << 1).max(4096);

                    continue;
                }
            }
            let (tx, mut rx) = ws_stream.split();

            // spawn the new sender task before notifying clients of the reconnect
            let msg_sender = runtime.spawn(msg_sender(msg_recv.clone(), tx));
            tokio::task::yield_now().await;

            info!("Successfully connected to caosim stream");
            state.store(ConnectionState::Online, Ordering::Release);
            reconnect_sender.send(Connected).unwrap();

            let entities_sender = entities_sender.clone();
            let terrain_sender = terrain_sender.clone();
            while let Some(msg) = rx.next().await {
                match msg {
                    Ok(tungstenite::Message::Text(txt)) => {
                        debug!("Incoming message");
                        let msg = serde_json::from_str::<cao_sim_model::Message>(txt.as_str())
                            .expect("Failed to deserialize msg");
                        match msg {
                            cao_sim_model::Message::Terrain(Some(terrain)) => {
                                info!("Got terrain");
                                let pl = terrain_model::terrain_payload_to_components(
                                    terrain.tiles.as_slice(),
                                )
                                .collect();

                                terrain_sender
                                    .send(NewTerrain {
                                        room_id: terrain.room_id,
                                        terrain: Arc::new(pl),
                                    })
                                    .expect("Failed to send new terrain");
                            }
                            cao_sim_model::Message::Terrain(None) => {
                                info!("Terrain request returned null");
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
                        msg_send
                            .send(tungstenite::Message::Pong(vec![]))
                            .unwrap_or_default();
                    }
                    Ok(msg) => {
                        debug!("Unexpected message variant {:?}", msg);
                    }
                    Err(err) => {
                        info!("Connection dropped ({}), reconnecting", err);
                    }
                }
            }
            state.store(ConnectionState::Closed, Ordering::Release);
            msg_sender.abort(); // abort this future, otherwise we might send events to it it can not handle in the future
        }
    });
}

impl Plugin for CaoSimPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let client = CaoClient::new();
        app.add_startup_system(setup.system())
            .add_event::<NewEntities>()
            .add_event::<NewTerrain>()
            .add_event::<Connected>()
            .add_system(send_new_entities_system.system())
            .add_system(send_new_terrain_system.system())
            .add_system(send_connected_event_system.system())
            .insert_resource(NewEntitiesRcv(client.on_new_entities.1.clone()))
            .insert_resource(NewTerrainRcv(client.on_new_terrain.1.clone()))
            .insert_resource(ConnectedRcv(client.on_connected.1.clone()))
            .insert_resource(MessageSender(client.send_message.0.clone()))
            .insert_resource(client)
            .insert_resource(ConnectionStateRes::new(ConnectionState::Connecting));
    }
}
