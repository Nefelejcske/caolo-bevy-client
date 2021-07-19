pub mod structure_assets;

use crate::{
    bots::pos_2d_to_3d,
    cao_sim_client::{cao_sim_model::AxialPos, hex_axial_to_pixel, NewEntities, SimEntityId},
};
use bevy::{
    ecs::system::EntityCommands,
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph,
    },
};
use std::collections::HashMap;

pub struct StructurePayload(
    pub HashMap<SimEntityId, crate::cao_sim_client::cao_sim_model::Structure>,
);
pub struct StructureIdMap(pub HashMap<SimEntityId, Entity>);
pub struct EntityPositionMap(pub HashMap<AxialPos, (SimEntityId, Entity)>);

pub struct Structure;

pub struct StructuresPlugin;

fn spawn_structure<'a, 'b>(
    cmd: &'b mut Commands<'a>,
    pos: Vec2,
    assets: &structure_assets::StructureRenderingAssets,
    materials: &mut Assets<structure_assets::StructureMaterial>,
) -> EntityCommands<'a, 'b> {
    let material = materials.add(structure_assets::StructureMaterial {
        color: Color::rgb(0.2, 0.3, 0.9),
        time: 0.0,
    });

    let mut cmd = cmd.spawn_bundle((
        Structure,
        Transform::from_translation(pos_2d_to_3d(pos)),
        GlobalTransform::default(),
    ));
    cmd.with_children(|c| {
        c.spawn_bundle(MeshBundle {
            mesh: assets.mesh.clone_weak(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                assets.pipeline.clone_weak(),
            )]),
            ..Default::default()
        })
        .insert(material);
    });
    cmd
}

fn update_materials_system(
    time: Res<Time>,
    mut materials: ResMut<Assets<structure_assets::StructureMaterial>>,
    query: Query<&Handle<structure_assets::StructureMaterial>>,
) {
    query.for_each_mut(move |handle| {
        if let Some(mat) = materials.get_mut(&*handle) {
            mat.time = time.seconds_since_startup() as f32;
        }
    });
}

fn on_new_entities_system(
    mut cmd: Commands,
    mut entity_map: ResMut<StructureIdMap>,
    mut positions: ResMut<EntityPositionMap>,
    mut payload: ResMut<StructurePayload>,
    assets: Res<structure_assets::StructureRenderingAssets>,
    mut materials: ResMut<Assets<structure_assets::StructureMaterial>>,
    mut new_entities: EventReader<NewEntities>,
    mut res_q: Query<&mut Transform, With<Structure>>,
) {
    for new_entities in new_entities.iter() {
        let len = entity_map.0.len();
        let mut prev = std::mem::replace(&mut entity_map.0, HashMap::with_capacity(len));
        let curr = &mut entity_map.0;
        curr.clear();
        positions.0.clear();
        payload.0.clear();
        for structure in new_entities.0.structures.iter() {
            let cao_id = SimEntityId(structure.id);
            let structure_id;
            if let Some(id) = prev.remove(&cao_id) {
                trace!("found entity {:?}", structure.id);
                curr.insert(cao_id, id);
                let mut tr = res_q
                    .get_mut(id)
                    .expect("Failed to query existing structure transform");
                // when structures respawn they usually aren't destroyed, just re-transformed
                let pos = &structure.pos;
                let pos = hex_axial_to_pixel(pos.q as f32, pos.r as f32);
                tr.translation = pos_2d_to_3d(pos);
                structure_id = id;
            } else {
                let pos = &structure.pos;
                let new_id = spawn_structure(
                    &mut cmd,
                    hex_axial_to_pixel(pos.q as f32, pos.r as f32),
                    &*assets,
                    &mut *materials,
                )
                .id();

                curr.insert(cao_id, new_id);
                trace!("new entity {:?}", structure.id);
                structure_id = new_id;
            }
            positions.0.insert(structure.pos, (cao_id, structure_id));
            payload.0.insert(cao_id, structure.clone());
        }
        // these entities were not sent in the current tick
        for (_, dead_entity) in prev {
            trace!("Entity {:?} died", dead_entity);
            cmd.entity(dead_entity).despawn_recursive();
        }
    }
}

fn setup_system(
    asset_server: Res<AssetServer>,
    mut pipelines: ResMut<Assets<bevy::render::pipeline::PipelineDescriptor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<render_graph::RenderGraph>,
    mut structure_rendering_assets: ResMut<structure_assets::StructureRenderingAssets>,
) {
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(
        bevy::render::shader::ShaderStages {
            vertex: asset_server.load::<Shader, _>("shaders/structure.vert"),
            fragment: Some(asset_server.load::<Shader, _>("shaders/structure.frag")),
        },
    ));
    render_graph.add_system_node(
        "structure_material",
        render_graph::AssetRenderResourcesNode::<structure_assets::StructureMaterial>::new(true),
    );
    render_graph
        .add_node_edge("structure_material", render_graph::base::node::MAIN_PASS)
        .unwrap();
    let mesh = meshes.add(Mesh::from(shape::Torus {
        radius: 0.5,
        ring_radius: 0.3,
        subdivisions_segments: 8,
        subdivisions_sides: 8,
    }));
    *structure_rendering_assets = structure_assets::StructureRenderingAssets {
        pipeline: pipeline_handle,
        mesh,
    };
}

impl Plugin for StructuresPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup_system.system())
            .add_system(on_new_entities_system.system())
            .add_system(update_materials_system.system())
            .init_resource::<structure_assets::StructureRenderingAssets>()
            .add_asset::<structure_assets::StructureMaterial>()
            .insert_resource(StructureIdMap(HashMap::with_capacity(1024)))
            .insert_resource(StructurePayload(HashMap::with_capacity(1024)))
            .insert_resource(EntityPositionMap(HashMap::with_capacity(1024)));
    }
}
