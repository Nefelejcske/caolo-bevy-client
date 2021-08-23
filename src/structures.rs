pub mod structure_assets;

use bevy::{
    ecs::system::EntityCommands,
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph,
    },
};

use crate::cao_entities::{pos_2d_to_3d, EntityMetadata, NewEntityEvent};

pub struct Structure;

pub struct StructuresPlugin;

fn build_structure(
    cmd: &mut EntityCommands,
    pos: Vec2,
    assets: &structure_assets::StructureRenderingAssets,
    materials: &mut Assets<structure_assets::StructureMaterial>,
) {
    let material = materials.add(structure_assets::StructureMaterial {
        color: Color::rgb(0.2, 0.3, 0.9),
        time: 0.0,
    });

    cmd.insert_bundle((
        Structure,
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
    });
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
    assets: Res<structure_assets::StructureRenderingAssets>,
    mut materials: ResMut<Assets<structure_assets::StructureMaterial>>,
    mut new_entities: EventReader<NewEntityEvent>,
    q_meta: Query<&EntityMetadata>,
) {
    for new_entity_event in new_entities
        .iter()
        .filter(|e| e.ty == crate::cao_entities::EntityType::Structure)
    {
        let meta = q_meta.get(new_entity_event.id).unwrap();
        build_structure(
            &mut cmd.entity(meta.id),
            meta.pos.as_pixel(),
            &*assets,
            &mut *materials,
        );
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
            .add_asset::<structure_assets::StructureMaterial>();
    }
}
