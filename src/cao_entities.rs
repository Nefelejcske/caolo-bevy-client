use std::collections::HashMap;

use bevy::{ecs::system::EntityCommands, prelude::*};

use crate::{
    cao_sim_client::{
        cao_sim_model::{AxialPos, EntityPosition},
        SimEntityId,
    },
    terrain::{is_room_visible, CurrentRoom, Room},
};
use lru::LruCache;

/// maps absolute coordinates to entity ids
pub struct EntityPositionMap(pub HashMap<AxialPos, smallvec::SmallVec<[Entity; 4]>>);
pub struct SimToBevyId(pub LruCache<SimEntityId, Entity>);
/// Latest time sent by the entities payload
pub struct LatestTime(pub i64);

#[derive(Debug, Clone, Copy)]
pub struct NewEntityEvent {
    pub id: Entity,
    pub cao_id: SimEntityId,
    pub ty: EntityType,
}

#[derive(Debug, Clone, Copy)]
pub struct EntityMovedEvent {
    pub id: Entity,
    pub cao_id: SimEntityId,
    pub ty: EntityType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    Bot,
    Structure,
    Resource,
}

#[derive(Debug, Clone)]
pub struct EntityMetadata {
    pub ty: EntityType,
    pub id: Entity,
    pub cao_id: SimEntityId,
    /// timestamp of the metadata
    pub ts: i64,
}

#[inline]
pub fn pos_2d_to_3d(p: Vec2) -> Vec3 {
    Vec3::new(p.x, 0.0, p.y)
}

/// deletes old entities
///
/// entities not in the meta-map are not deleted by this system
// TODO: once entity death events  are available use those + check the visible rooms maybe
fn entity_gc_system(
    mut cmd: Commands,
    latest: Res<LatestTime>,
    sim2bevy: Res<SimToBevyId>,
    current_room: Res<CurrentRoom>,
    q: Query<(Entity, &SimEntityId, &EntityPosition, &EntityMetadata)>,
) {
    let current_time = latest.0;
    for (e, se, wp, meta) in q.iter() {
        if current_time - meta.ts > 3 {
            trace!("Deleting expired entity {:?}", se);
            cmd.entity(e).despawn_recursive();
        } else if !sim2bevy.0.contains(se) {
            trace!("Deleting dead entity {:?}", se);
            cmd.entity(e).despawn_recursive();
        } else if !is_room_visible(&*current_room, &Room(wp.room)) {
            trace!("Deleting out of range entity {:?}", se);
            cmd.entity(e).despawn_recursive();
        }
    }
}

fn handle_new_entity<'a, 'b>(
    time: i64,
    cmd: &'b mut Commands<'a>,
    cao_id: SimEntityId,
    ty: EntityType,
    wp: EntityPosition,
    moved_event: &mut EventWriter<EntityMovedEvent>,
    spawned_event: &mut EventWriter<NewEntityEvent>,
    meta_map: &mut Query<(&mut EntityMetadata, &mut EntityPosition)>,
    sim2bevy: &mut SimToBevyId,
) -> EntityCommands<'a, 'b> {
    let entity_id;
    if let Some((id, (mut metadata, mut world_pos))) = sim2bevy
        .0
        .get(&cao_id) // moves this entity to the top of the LRU
        .and_then(|id| meta_map.get_mut(*id).ok().map(|x| (id, x)))
        // if the simulation recycled this ID, we treat it as a new entity
        .and_then(|(id, m)| (m.0.ty == ty).then(|| (id, m)))
    {
        debug_assert_eq!(metadata.cao_id, cao_id);
        entity_id = *id;

        trace!("found entity {:?}", metadata.cao_id);

        if *world_pos != wp {
            moved_event.send(EntityMovedEvent {
                id: entity_id,
                cao_id,
                ty,
            });
            *world_pos = wp.clone();
        }
        metadata.ts = time;

        cmd.entity(metadata.id)
    } else {
        // spawn new entity
        //
        let mut cmd = cmd.spawn();
        cmd.insert_bundle((cao_id, ty, wp.clone()));
        entity_id = cmd.id();

        let meta = EntityMetadata {
            ty,
            id: entity_id,
            cao_id,
            ts: time,
        };

        trace!("new entity {:?}", meta);
        cmd.insert(meta);

        sim2bevy.0.put(cao_id, entity_id);
        moved_event.send(EntityMovedEvent {
            id: entity_id,
            cao_id,
            ty,
        });
        spawned_event.send(NewEntityEvent {
            id: entity_id,
            cao_id,
            ty,
        });
        cmd
    }
}

fn update_positions_system(
    mut positions_map: ResMut<EntityPositionMap>,
    q: Query<(Entity, &EntityPosition)>,
) {
    positions_map.0.clear();
    for (e, pos) in q.iter() {
        positions_map
            .0
            .entry(pos.absolute_axial())
            .or_default()
            .push(e);
    }
}

fn on_new_entities_system(
    mut cmd: Commands,
    mut new_entities: EventReader<crate::cao_sim_client::NewEntities>,
    mut moved_event: EventWriter<EntityMovedEvent>,
    mut spawned_event: EventWriter<NewEntityEvent>,
    mut latest_ts: ResMut<LatestTime>,
    mut sim2bevy: ResMut<SimToBevyId>,
    mut meta_map: Query<(&mut EntityMetadata, &mut EntityPosition)>,
) {
    for new_entities in new_entities.iter() {
        let time = new_entities.0.time;

        latest_ts.0 = latest_ts.0.max(time);

        let cmd = &mut cmd;
        let moved_event = &mut moved_event;
        let spawned_event = &mut spawned_event;

        for structure in new_entities.0.structures.iter() {
            let cao_id = SimEntityId(structure.id);
            handle_new_entity(
                time,
                cmd,
                cao_id,
                EntityType::Structure,
                structure.pos.clone(),
                moved_event,
                spawned_event,
                &mut meta_map,
                &mut *sim2bevy,
            )
            .insert(structure.clone());
        }
        for resource in new_entities.0.resources.iter() {
            let cao_id = SimEntityId(resource.id);
            handle_new_entity(
                time,
                cmd,
                cao_id,
                EntityType::Resource,
                resource.pos.clone(),
                moved_event,
                spawned_event,
                &mut meta_map,
                &mut *sim2bevy,
            )
            .insert(resource.clone());
        }
        for bot in new_entities.0.bots.iter() {
            let cao_id = SimEntityId(bot.id);
            handle_new_entity(
                time,
                cmd,
                cao_id,
                EntityType::Bot,
                bot.pos.clone(),
                moved_event,
                spawned_event,
                &mut meta_map,
                &mut *sim2bevy,
            )
            .insert(bot.clone());
        }
    }
}

pub struct CaoEntityPlugin;

impl Plugin for CaoEntityPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(EntityPositionMap(HashMap::with_capacity(2048)))
            .insert_resource(SimToBevyId(LruCache::new(4096)))
            .insert_resource(LatestTime(-1))
            .add_event::<NewEntityEvent>()
            .add_event::<EntityMovedEvent>()
            .add_system(update_positions_system.system())
            .add_stage_before(
                CoreStage::PreUpdate,
                "remote_input",
                SystemStage::parallel(),
            )
            .add_stage_after(CoreStage::PostUpdate, "gc", SystemStage::parallel())
            .add_system_to_stage("remote_input", on_new_entities_system.system())
            .add_system_to_stage("gc", entity_gc_system.system());
    }
}
