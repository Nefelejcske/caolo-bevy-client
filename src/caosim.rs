//! Components / Systems related to interacting with the remote Simulation
//!
mod room;

use bevy::prelude::*;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use self::room::{
    fetch_world, set_room, CaoEntityId, CaoWorldState, CurrentRoom, FetchWorldTimer, NewRoomState,
    SetRoom,
};

#[derive(serde::Deserialize, Debug, Clone, Default, Copy)]
pub struct HexPos {
    q: i32,
    r: i32,
}
#[derive(Debug, Clone, Default, Copy)]
pub struct Bot;

fn on_new_room(
    mut cmd: Commands,
    room: Res<CurrentRoom>,
    mut new_room_event: ResMut<Events<NewRoomState>>,
    current_entities: Query<(Entity, &CaoEntityId, &Bot, &HexPos)>,
) {
    let mut dr = new_room_event.drain();
    // `room` already contains the latest state, we only care if there are any timestamps
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
