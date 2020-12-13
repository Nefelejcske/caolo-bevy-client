use bevy::{prelude::*, tasks::IoTaskPool};

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tracing::debug;

use super::HexPos;

/// Time
#[derive(Debug, Clone, Copy)]
pub struct NewRoomState(pub i64);

#[derive(Debug, Clone)]
pub struct CurrentRoom(pub CaoWorldState);

#[derive(serde::Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct CaoWorldPos {
    pub room: HexPos,
    pub room_pos: HexPos,
}

#[derive(Debug, Clone, Default, Copy, Eq, PartialEq, Hash)]
pub struct CaoEntityId(pub i64);

#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct CaoBot {
    #[serde(rename = "__id")]
    pub id: i64,
    pub pos: CaoWorldPos,
}

#[derive(Debug, Clone, Default)]
pub struct CaoWorldState {
    pub time: i64,
    pub bots: HashMap<i64, CaoBot>,
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct CaoWorldRoomDe {
    pub time: i64,
    pub payload: CaoWorldRoomPayload,
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct CaoWorldRoomPayload {
    pub bots: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
}

pub struct SetRoom(pub Arc<Mutex<Option<CaoWorldRoomDe>>>);

pub struct FetchWorldTimer(pub Timer);

pub fn fetch_world(
    mut timer: ResMut<FetchWorldTimer>,
    setter: Res<SetRoom>,
    time: Res<Time>,
    pool: Res<IoTaskPool>,
) {
    timer.0.tick(time.delta_seconds);

    if timer.0.finished {
        debug!("Fetching world");
        let setter = Arc::clone(&setter.0);
        pool.spawn(async move {
            let response: CaoWorldRoomDe =
                surf::get("https://caolo.herokuapp.com/room-objects?q=15&r=18")
                    .recv_json()
                    .await
                    .expect("Failed to get");
            let mut setter = setter.lock().unwrap();
            *setter = Some(response);
        })
        .detach();
    }
}

pub fn set_room(
    new_room: Res<SetRoom>,
    room: ResMut<CurrentRoom>,
    new_room_event: ResMut<Events<NewRoomState>>,
) {
    if let Some(r) = new_room.0.lock().unwrap().take() {
        _set_room(r, room, new_room_event);
    }
}

fn _set_room(
    mut r: CaoWorldRoomDe,
    mut room: ResMut<CurrentRoom>,
    mut new_room_event: ResMut<Events<NewRoomState>>,
) {
    debug!("Setting new room state");

    if r.time != room.0.time {
        new_room_event.send(NewRoomState(r.time));
    }
    room.0.time = r.time;
    room.0.bots = r
        .payload
        .bots
        .take()
        .map(|bots| {
            bots.into_iter()
                .map(|bot| {
                    let id = bot["__id"].as_i64().unwrap();
                    (
                        id,
                        CaoBot {
                            pos: serde_json::from_value(bot["pos"].clone()).unwrap(),
                            id,
                        },
                    )
                })
                .collect()
        })
        .unwrap_or_else(Default::default);
}
