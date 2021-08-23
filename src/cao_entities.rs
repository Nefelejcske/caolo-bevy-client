use std::collections::HashMap;

use bevy::{ecs::system::EntityCommands, prelude::*};

use crate::cao_sim_client::{
    cao_sim_model::{AxialPos, WorldPosition},
    SimEntityId,
};

/// maps absolute coordinates to entity ids
pub struct EntityPositionMap(pub HashMap<AxialPos, smallvec::SmallVec<[Entity; 4]>>);
pub struct SimToBevyId(pub HashMap<SimEntityId, Entity>);
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
    pub pos: WorldPosition,
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
    ts: Res<LatestTime>,
    q: Query<(Entity, &SimEntityId, &EntityMetadata)>,
    mut positions_map: ResMut<EntityPositionMap>,
) {
    let latest_timestamp = ts.0;
    for (e, se, meta) in q.iter() {
        if (latest_timestamp - meta.ts) >= 2 {
            trace!("Deleting dead entity {:?}", se);
            cmd.entity(e).despawn_recursive();
            if let Some(poss) = positions_map.0.get_mut(&meta.pos.absolute_axial()) {
                if let Some((idx, _)) = poss.iter().enumerate().find(|(_, id)| id == &&e) {
                    poss.swap_remove(idx);
                }
            }
        }
    }
}

fn handle_new_entity<'a, 'b>(
    time: i64,
    cmd: &'b mut Commands<'a>,
    cao_id: SimEntityId,
    ty: EntityType,
    wp: WorldPosition,
    positions_map: &mut EntityPositionMap,
    moved_event: &mut EventWriter<EntityMovedEvent>,
    spawned_event: &mut EventWriter<NewEntityEvent>,
    meta_map: &mut Query<&mut EntityMetadata>,
    sim2bevy: &mut SimToBevyId,
) -> EntityCommands<'a, 'b> {
    let entity_id;
    let cmd = if let Some((id, mut metadata)) = sim2bevy
        .0
        .get(&cao_id)
        .and_then(|id| meta_map.get_mut(*id).ok().map(|x| (id, x)))
        // if the simulation recycled this ID, we treat it as a new entity
        .and_then(|(id, m)| (m.ty == ty).then(|| (id, m)))
    {
        debug_assert_eq!(metadata.cao_id, cao_id);
        entity_id = *id;

        trace!("found entity {:?}", metadata.cao_id);

        if metadata.pos != wp {
            if let Some(ids) = positions_map.0.get_mut(&metadata.pos.absolute_axial()) {
                if let Some((idx, _)) = ids.iter().enumerate().find(|(_, id)| id == &&metadata.id) {
                    ids.swap_remove(idx);
                }
            }
            moved_event.send(EntityMovedEvent {
                id: entity_id,
                cao_id,
                ty,
            });
        }
        metadata.pos = wp.clone();
        metadata.ts = time;

        cmd.entity(metadata.id)
    } else {
        let mut cmd = cmd.spawn();
        cmd.insert(cao_id).insert(ty);
        entity_id = cmd.id();

        let meta = EntityMetadata {
            ty,
            id: entity_id,
            cao_id,
            pos: wp.clone(),
            ts: time,
        };

        trace!("new entity {:?}", meta);
        cmd.insert(meta);

        sim2bevy.0.insert(cao_id, entity_id);
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
    };
    positions_map
        .0
        .entry(wp.absolute_axial())
        .or_default()
        .push(entity_id);
    cmd
}

fn on_new_entities_system(
    mut cmd: Commands,
    mut new_entities: EventReader<crate::cao_sim_client::NewEntities>,
    mut positions_map: ResMut<EntityPositionMap>,
    mut moved_event: EventWriter<EntityMovedEvent>,
    mut spawned_event: EventWriter<NewEntityEvent>,
    mut latest_ts: ResMut<LatestTime>,
    mut sim2bevy: ResMut<SimToBevyId>,
    mut meta_map: Query<&mut EntityMetadata>,
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
                &mut *positions_map,
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
                &mut *positions_map,
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
                &mut *positions_map,
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
            .insert_resource(SimToBevyId(HashMap::with_capacity(2048)))
            .insert_resource(LatestTime(-1))
            .add_event::<NewEntityEvent>()
            .add_event::<EntityMovedEvent>()
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
