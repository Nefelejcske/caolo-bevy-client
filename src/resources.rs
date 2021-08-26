pub mod resource_assets;

use bevy::{
    ecs::system::EntityCommands,
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph,
    },
};

use crate::{
    cao_entities::{pos_2d_to_3d, EntityMetadata, EntityMovedEvent, NewEntityEvent},
    cao_sim_client::cao_sim_model::{self, EntityPosition},
};

pub struct Resource;

pub struct ResourcesPlugin;

fn build_resource(
    cmd: &mut EntityCommands,
    pos: Vec2,
    assets: &resource_assets::ResourceRenderingAssets,
    materials: &mut Assets<resource_assets::ResourceMaterial>,
) -> Entity {
    let material = materials.add(resource_assets::ResourceMaterial {
        color: Color::rgb(0.2, 0.2, 0.8),
        time: 0.0,
    });

    cmd.insert_bundle((
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
    assets: Res<resource_assets::ResourceRenderingAssets>,
    mut materials: ResMut<Assets<resource_assets::ResourceMaterial>>,
    mut new_entities: EventReader<NewEntityEvent>,
    q_meta: Query<(&EntityMetadata, &EntityPosition)>,
) {
    for new_entity_event in new_entities
        .iter()
        .filter(|e| e.ty == crate::cao_entities::EntityType::Resource)
    {
        let (meta, wp) = q_meta.get(new_entity_event.id).unwrap();
        build_resource(
            &mut cmd.entity(meta.id),
            wp.as_pixel(),
            &*assets,
            &mut *materials,
        );
    }
}

fn on_resource_move_system(
    mut moved_entities: EventReader<EntityMovedEvent>,
    mut res_data: Query<(&cao_sim_model::Resource, &EntityPosition, &mut Transform)>,
) {
    for event in moved_entities
        .iter()
        .filter(|m| m.ty == crate::cao_entities::EntityType::Resource)
    {
        let (_res, pos, mut tr) = match res_data.get_mut(event.id) {
            Ok(b) => b,
            Err(err) => {
                trace!(
                    "Received resource moved event but the entity can't be queried {:?}",
                    err
                );
                continue;
            }
        };
        tr.translation = pos_2d_to_3d(pos.as_pixel());
    }
}

fn setup(
    asset_server: Res<AssetServer>,
    mut pipelines: ResMut<Assets<bevy::render::pipeline::PipelineDescriptor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<render_graph::RenderGraph>,
    mut resource_rendering_assets: ResMut<resource_assets::ResourceRenderingAssets>,
) {
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
            .add_system_set(
                SystemSet::on_update(crate::AppState::Room)
                    .with_system(on_new_entities.system())
                    .with_system(on_resource_move_system.system())
                    .with_system(update_res_materials.system()),
            )
            .add_asset::<resource_assets::ResourceMaterial>()
            .init_resource::<resource_assets::ResourceRenderingAssets>();
    }
}
