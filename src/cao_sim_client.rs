//! Components / Systems related to interacting with the remote Simulation
//!

pub mod cao_client;
pub mod cao_sim_model;
pub mod terrain_model;

use anyhow::Context;
use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};
use cao_sim_model::GetLayoutQuery;
use futures::prelude::*;
use futures_lite::future;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use self::{
    cao_client::CaoClient,
    cao_sim_model::{AxialPos, TerrainTy},
};

pub struct CaoSimPlugin;
pub struct NewEntities(pub Arc<cao_sim_model::EntitiesPayload>);
pub struct NewTerrain {
    pub room_id: AxialPos,
    pub offset: AxialPos,
    pub terrain: Arc<Vec<(AxialPos, TerrainTy)>>,
}
pub struct Connected;
pub struct TerrainLayout(pub Vec<AxialPos>);

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

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SimEntityId(pub u64);

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

fn handle_message(
    txt: &str,
    terrain_sender: &crossbeam::channel::Sender<NewTerrain>,
    entities_sender: &crossbeam::channel::Sender<NewEntities>,
    layout: &[AxialPos],
) -> anyhow::Result<()> {
    trace!("Incoming message");
    let msg = serde_json::from_str::<cao_sim_model::Message>(txt)
        .with_context(|| "Failed to deserialize msg")?;
    match msg {
        cao_sim_model::Message::Terrain(Some(terrain)) => {
            info!(
                "Got terrain for room: {:?}, offset: {:?}",
                terrain.room_id, terrain.offset
            );
            let pl = terrain_model::terrain_payload_to_components(terrain.tiles.as_slice(), layout)
                .collect();

            terrain_sender
                .send(NewTerrain {
                    room_id: terrain.room_id,
                    offset: terrain.offset,
                    terrain: Arc::new(pl),
                })
                .with_context(|| "Failed to send new terrain")?;
        }
        cao_sim_model::Message::Terrain(None) => {
            info!("Terrain request returned null");
        }
        cao_sim_model::Message::Entities(ent) => {
            trace!("New entities, time: {}, room: {:?}", ent.time, ent.room_id);
            entities_sender
                .send(NewEntities(Arc::new(ent)))
                .with_context(|| "Failed to send new entities")?;
        }
    }
    Ok(())
}

async fn listen_to_cao_rt(
    layout: Vec<AxialPos>,
    state: ConnectionStateRes,
    runtime: Arc<tokio::runtime::Runtime>,
    msg_recv: crossbeam::channel::Receiver<tungstenite::Message>,
    msg_send: crossbeam::channel::Sender<tungstenite::Message>,
    entities_sender: crossbeam::channel::Sender<NewEntities>,
    terrain_sender: crossbeam::channel::Sender<NewTerrain>,
    reconnect_sender: crossbeam::channel::Sender<Connected>,
) {
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
                    if let Err(err) = handle_message(
                        txt.as_str(),
                        &terrain_sender,
                        &entities_sender,
                        layout.as_slice(),
                    ) {
                        error!("Failed to handle message {:?}", err);
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
        msg_sender.abort(); // abort this future, otherwise we might send events to it that it can not handle in the future
    }
}

fn handle_tasks_system(
    mut commands: Commands,
    mut layout: ResMut<TerrainLayout>,
    q: Query<(Entity, &mut Task<TerrainLayout>)>,
    //
    client: Res<CaoClient>,
    state: Res<ConnectionStateRes>,
) {
    q.for_each_mut(|(e, mut t)| {
        if let Some(stuff) = future::block_on(future::poll_once(&mut *t)) {
            *layout = stuff;
            commands.entity(e).remove::<Task<TerrainLayout>>();

            client.runtime.spawn(listen_to_cao_rt(
                layout.0.clone(),
                state.clone(),
                client.runtime.clone(),
                client.send_message.1.clone(),
                client.send_message.0.clone(),
                client.on_new_entities.0.clone(),
                client.on_new_terrain.0.clone(),
                client.on_connected.0.clone(),
            ));
        }
    });
}

async fn get_layout(q: &GetLayoutQuery) -> Vec<AxialPos> {
    surf::get(format!("{}/world/room-terrain-layout", crate::API_BASE_URL))
        .query(q)
        .expect("Failed to set query param")
        .recv_json()
        .await
        .expect("Failed to get layout")
}

/// Fire NewEntities event and reset current_entities
fn send_new_entities_system(
    recv: Res<NewEntitiesRcv>,
    mut on_new_entities: EventWriter<NewEntities>,
) {
    while let Ok(entities) = recv.0.recv_timeout(Duration::from_micros(1)) {
        let entities = entities.0;
        on_new_entities.send(NewEntities(entities.clone()));
    }
}

async fn get_connection() -> Result<Ws, tungstenite::error::Error> {
    async_tungstenite::tokio::connect_async(
        format!("{}/object-stream", crate::WS_BASE_URL).as_str(),
    )
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

fn setup_layout_task_system(mut commands: Commands, task_pool: Res<IoTaskPool>) {
    let handle = task_pool.spawn(async move {
        let res = get_layout(&GetLayoutQuery {
            radius: 30, // TODO
        })
        .await;
        TerrainLayout(res)
    });

    commands.spawn().insert(handle);
}

impl Plugin for CaoSimPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let client = CaoClient::new();
        app.add_startup_system(setup_layout_task_system.system())
            .add_event::<NewEntities>()
            .add_event::<NewTerrain>()
            .add_event::<Connected>()
            .add_system(send_new_entities_system.system())
            .add_system(send_new_terrain_system.system())
            .add_system(send_connected_event_system.system())
            .add_system(handle_tasks_system.system())
            .insert_resource(TerrainLayout(Vec::with_capacity(10000)))
            .insert_resource(NewEntitiesRcv(client.on_new_entities.1.clone()))
            .insert_resource(NewTerrainRcv(client.on_new_terrain.1.clone()))
            .insert_resource(ConnectedRcv(client.on_connected.1.clone()))
            .insert_resource(MessageSender(client.send_message.0.clone()))
            .insert_resource(client)
            .insert_resource(ConnectionStateRes::new(ConnectionState::Connecting));
    }
}
