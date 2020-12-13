//! Components / Systems related to interacting with the remote Simulation
//!
mod bots;
mod sim;

use bevy::prelude::*;

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use self::sim::{
    fetch_world, set_room, CaoEntityId, CaoWorldState, CurrentRoom, FetchWorldTimer, NewRoomState,
    SetRoom,
};

#[derive(serde::Deserialize, Debug, Clone, Default, Copy, Eq, PartialEq)]
pub struct HexPos {
    q: i32,
    r: i32,
}
#[derive(Debug, Clone, Default, Copy)]
pub struct Bot;

fn on_new_room(
    mut cmd: Commands,
    mut materials: ResMut<Assets<bots::BotMaterial>>,
    room: Res<CurrentRoom>,
    assets: Res<bots::resources::BotRenderingAssets>,
    mut new_room_event: ResMut<Events<NewRoomState>>,
    current_entities: Query<(Entity, &CaoEntityId, &Bot)>,
    mut room_pos: Query<Mut<HexPos>>,
) {
    let mut dr = new_room_event.drain();
    // `room` already contains the latest state, we only care if there are any timestamps
    if dr.next().is_some() {
        // update the entities
        let mut seen: HashSet<CaoEntityId> = HashSet::new();
        for (entity, cao_entity, ..) in current_entities.iter() {
            // first check the current entities and update them as appropriate
            seen.insert(*cao_entity);
            match room.0.bots.get(&cao_entity.0) {
                Some(caobot) => {
                    let mut hex = room_pos
                        .get_mut(entity)
                        .expect("Failec to get the HexPos of entity");
                    *hex = caobot.pos.room_pos;
                }
                None => {
                    cmd.despawn_recursive(entity);
                }
            }
        }
        // spawn new entities
        for (_, cao_bot) in room
            .0
            .bots
            .iter()
            .filter(|(_, bot)| !seen.contains(&CaoEntityId(bot.id)))
        {
            bots::spawn_bot(&mut cmd, cao_bot, &*assets, &mut *materials);
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
            .add_resource(bots::resources::BotRenderingAssets::default())
            .add_asset::<bots::BotMaterial>()
            .add_startup_system(bots::setup.system())
            .add_system(fetch_world.system())
            .add_system(on_new_room.system())
            .add_system(bots::update_target_pos.system())
            .add_system(bots::update_current_pos.system())
            .add_system(set_room.system());
    }
}
