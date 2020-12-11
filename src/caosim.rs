use bevy::{prelude::*, tasks::IoTaskPool};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

struct FetchWorldTimer(Timer);

#[derive(serde::Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct CaoWorldPos {
    room: HexPos,
    room_pos: HexPos,
}
#[derive(serde::Deserialize, Debug, Clone, Default, Copy)]
pub struct HexPos {
    q: i32,
    r: i32,
}
#[derive(Debug, Clone, Default, Copy)]
pub struct Bot;
#[derive(Debug, Clone, Default, Copy, Eq, PartialEq, Hash)]
pub struct CaoEntityId(i64);

/// Time
struct NewRoomState(i64);

#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct CaoBot {
    #[serde(rename = "__id")]
    id: i64,
    pos: CaoWorldPos,
}

#[derive(Debug, Clone, Default)]
pub struct CaoWorldState {
    pub time: i64,
    pub bots: HashMap<i64, CaoBot>,
}

#[derive(Debug, Clone)]
pub struct CurrentRoom(CaoWorldState);

#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct CaoWorldRoomDe {
    pub time: i64,
    pub payload: CaoWorldRoomPayload,
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct CaoWorldRoomPayload {
    pub bots: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
}

pub struct SetRoom(Arc<Mutex<Option<CaoWorldRoomDe>>>);

fn fetch_world(
    mut timer: ResMut<FetchWorldTimer>,
    setter: Res<SetRoom>,
    time: Res<Time>,
    pool: Res<IoTaskPool>,
) {
    timer.0.tick(time.delta_seconds);

    if timer.0.finished {
        let setter = Arc::clone(&setter.0);
        pool.spawn(async move {
            let response: CaoWorldRoomDe =
                surf::get("https://caolo.herokuapp.com/room-objects?q=16&r=17")
                    .recv_json()
                    .await
                    .expect("Failed to get");
            let mut setter = setter.lock().unwrap();
            *setter = Some(response);
        })
        .detach();
    }
}

fn set_room(
    new_room: Res<SetRoom>,
    mut room: ResMut<CurrentRoom>,
    mut new_room_event: ResMut<Events<NewRoomState>>,
) {
    if let Some(mut r) = new_room.0.lock().unwrap().take() {
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
}

fn on_new_room(
    mut cmd: Commands,
    room: Res<CurrentRoom>,
    mut new_room_event: ResMut<Events<NewRoomState>>,
    current_entities: Query<(Entity, &CaoEntityId, &Bot, &HexPos)>,
) {
    let mut dr = new_room_event.drain();
    if dr.next().is_some() {
        // update the entities
        let mut seen: HashSet<CaoEntityId> = HashSet::new();
        for (entity, cao_entity, ..) in current_entities.iter() {
            seen.insert(*cao_entity);
            match room.0.bots.get(&cao_entity.0) {
                Some(caobot) => {
                    cmd.insert_one(entity, caobot.pos.room_pos);
                }
                None => {
                    cmd.despawn_recursive(entity);
                }
            }
        }
        // spawn new entities
        for (_, cao_entity) in room
            .0
            .bots
            .iter()
            .filter(|(_, bot)| !seen.contains(&CaoEntityId(bot.id)))
        {
            cmd.spawn((CaoEntityId(cao_entity.id), Bot, cao_entity.pos.room_pos));
        }
    }
    // drain the rest
    // `room` already contains the latest state, we don't care about the other timestamps, if any
    for _ in dr {}
}

pub struct CaoSimPlugin;

impl Plugin for CaoSimPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource(FetchWorldTimer(Timer::from_seconds(1.0, true)))
            .add_resource(SetRoom(Arc::new(Mutex::new(None))))
            .add_resource(CurrentRoom(CaoWorldState::default()))
            .add_resource(Events::<NewRoomState>::default())
            .add_system(fetch_world.system())
            .add_system(on_new_room.system())
            .add_system(set_room.system());
    }
}
