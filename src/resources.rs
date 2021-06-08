pub mod resource_assets;

use std::collections::HashMap;

use bevy::{
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph,
    },
};

use crate::{
    bots::pos_2d_to_3d,
    caosim::{hex_axial_to_pixel, NewEntities, SimEntityId},
};

#[derive(Default)]
struct EntityMap(pub HashMap<SimEntityId, Entity>);

struct Resource;

pub struct ResourcesPlugin;

fn spawn_resource(
    cmd: &mut Commands,
    pos: Vec2,
    assets: &resource_assets::ResourceRenderingAssets,
    materials: &mut Assets<resource_assets::ResourceMaterial>,
) -> Entity {
    let material = materials.add(resource_assets::ResourceMaterial {
        color: Color::rgb(0.2, 0.2, 0.8),
        time: 0.0,
    });

    cmd.spawn_bundle((
        Resource,
        Transform::from_translation(pos_2d_to_3d(pos)),
        GlobalTransform::default(),
    ))
    .with_children(|c| {
        c.spawn_bundle(MeshBundle {
            mesh: assets.mesh.clone_weak(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                assets.pipeline.clone_weak(),
            )]),
            ..Default::default()
        })
        .insert(material);
    })
    .id()
}

fn update_res_materials(
    time: Res<Time>,
    mut materials: ResMut<Assets<resource_assets::ResourceMaterial>>,
    query: Query<&Handle<resource_assets::ResourceMaterial>>,
) {
    query.for_each_mut(move |handle| {
        if let Some(mat) = materials.get_mut(&*handle) {
            mat.time = time.seconds_since_startup() as f32;
        }
    });
}

fn on_new_entities(
    mut cmd: Commands,
    mut entity_map: ResMut<EntityMap>,
    assets: Res<resource_assets::ResourceRenderingAssets>,
    mut materials: ResMut<Assets<resource_assets::ResourceMaterial>>,
    mut new_entities: EventReader<NewEntities>,
    mut res_q: Query<&mut Transform, With<Resource>>,
) {
    for new_entities in new_entities.iter() {
        let len = entity_map.0.len();
        let mut prev = std::mem::replace(&mut entity_map.0, HashMap::with_capacity(len));
        let curr = &mut entity_map.0;
        curr.clear();
        for res in new_entities.0.resources.iter() {
            let cao_id = SimEntityId(res.id);
            if let Some(res_id) = prev.remove(&cao_id) {
                trace!("found entity {:?}", res.id);
                curr.insert(cao_id, res_id);
                let mut tr = res_q
                    .get_mut(res_id)
                    .expect("Failed to query existing resource transform");
                // when resources respawn they usually aren't destroyed, just re-transformed
                let pos = &res.pos;
                let pos = hex_axial_to_pixel(pos.q as f32, pos.r as f32);
                tr.translation = pos_2d_to_3d(pos);
            } else {
                let pos = &res.pos;
                let new_id = spawn_resource(
                    &mut cmd,
                    hex_axial_to_pixel(pos.q as f32, pos.r as f32),
                    &*assets,
                    &mut *materials,
                );

                curr.insert(cao_id, new_id);
                trace!("new entity {:?}", res.id);
            }
        }
        // these entities were not sent in the current tick
        for (_, dead_entity) in prev {
            trace!("Entity {:?} died", dead_entity);
            cmd.entity(dead_entity).despawn_recursive();
        }
    }
}

fn setup(
    asset_server: Res<AssetServer>,
    mut pipelines: ResMut<Assets<bevy::render::pipeline::PipelineDescriptor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<render_graph::RenderGraph>,
    mut resource_rendering_assets: ResMut<resource_assets::ResourceRenderingAssets>,
) {
    asset_server.watch_for_changes().unwrap();

    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(
        bevy::render::shader::ShaderStages {
            vertex: asset_server.load::<Shader, _>("shaders/resource.vert"),
            fragment: Some(asset_server.load::<Shader, _>("shaders/resource.frag")),
        },
    ));

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind resourceMaterial resources to our shader
    render_graph.add_system_node(
        "resource_material",
        render_graph::AssetRenderResourcesNode::<resource_assets::ResourceMaterial>::new(true),
    );

    // Add a Render Graph edge connecting our new "resource_material" node to the main pass node. This ensures "resource_material" runs before the main pass
    render_graph
        .add_node_edge("resource_material", render_graph::base::node::MAIN_PASS)
        .unwrap();

    let mesh = meshes.add(Mesh::from(shape::Icosphere {
        radius: 1.,
        subdivisions: 3,
    }));
    *resource_rendering_assets = resource_assets::ResourceRenderingAssets {
        pipeline: pipeline_handle,
        mesh,
    };
}

impl Plugin for ResourcesPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system(on_new_entities.system())
            .add_system(update_res_materials.system())
            .init_resource::<resource_assets::ResourceRenderingAssets>()
            .add_asset::<resource_assets::ResourceMaterial>()
            .init_resource::<EntityMap>();
    }
}
