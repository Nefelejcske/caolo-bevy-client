mod terrain_assets;

use bevy::{
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph,
    },
};

use crate::bots::pos_2d_to_3d;
use crate::caosim::{cao_sim_model::TerrainTy, hex_axial_to_pixel, NewTerrain};

pub struct TerrainPlugin;

pub struct TerrainTile(pub TerrainTy);

fn terrain2color(ty: TerrainTy) -> Color {
    match ty {
        TerrainTy::Empty => Color::rgb(0.0, 0.0, 0.0),
        TerrainTy::Plain => Color::rgb(0.5, 0.5, 0.0),
        TerrainTy::Wall => Color::rgb(0.8, 0.0, 0.0),
        TerrainTy::Bridge => Color::rgb(0.0, 0.8, 0.0),
    }
}

fn on_new_terrain(
    mut cmd: Commands,
    mut new_terrain: EventReader<NewTerrain>,
    assets: Res<terrain_assets::TerrainRenderingAssets>,
    mut materials: ResMut<Assets<terrain_assets::TerrainMaterial>>,
    existing_tiles: Query<Entity, With<TerrainTile>>,
) {
    for new_terrain in new_terrain.iter() {
        info!("Poggies {:?}", new_terrain.room_id);

        for e in existing_tiles.iter() {
            // TODO: maybe update these instead of despawning all...?
            cmd.entity(e).despawn_recursive();
        }

        for (hex_pos, ty) in new_terrain.terrain.iter() {
            let pos = hex_axial_to_pixel(hex_pos.q as f32, hex_pos.r as f32);
            let mut pos = pos_2d_to_3d(pos);
            pos.y -= 1.0;

            let material = materials.add(terrain_assets::TerrainMaterial {
                color: terrain2color(*ty),
            });

            cmd.spawn_bundle(MeshBundle {
                mesh: assets.mesh.clone_weak(),
                render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                    assets.pipeline.clone_weak(),
                )]),
                transform: Transform::from_translation(pos),
                ..Default::default()
            })
            .insert(material)
            .insert(TerrainTile(*ty));
        }
    }
}

fn setup(
    asset_server: Res<AssetServer>,
    mut pipelines: ResMut<Assets<bevy::render::pipeline::PipelineDescriptor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<render_graph::RenderGraph>,
    mut terrain_rendering_assets: ResMut<terrain_assets::TerrainRenderingAssets>,
) {
    asset_server.watch_for_changes().unwrap();

    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(
        bevy::render::shader::ShaderStages {
            vertex: asset_server.load::<Shader, _>("shaders/terrain.vert"),
            fragment: Some(asset_server.load::<Shader, _>("shaders/terrain.frag")),
        },
    ));

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind BotMaterial resources to our shader
    render_graph.add_system_node(
        "terrain_material",
        render_graph::AssetRenderResourcesNode::<terrain_assets::TerrainMaterial>::new(true),
    );

    // Add a Render Graph edge connecting our new "terrain_material" node to the main pass node. This ensures "terrain_material" runs before the main pass
    render_graph
        .add_node_edge("terrain_material", render_graph::base::node::MAIN_PASS)
        .unwrap();

    let mesh = meshes.add(Mesh::from(shape::Plane { size: 1.4 }));
    *terrain_rendering_assets = terrain_assets::TerrainRenderingAssets {
        pipeline: pipeline_handle,
        mesh,
    };
}

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system(on_new_terrain.system())
            .init_resource::<terrain_assets::TerrainRenderingAssets>()
            .add_asset::<terrain_assets::TerrainMaterial>();
    }
}
