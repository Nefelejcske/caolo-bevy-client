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

pub struct Room;

fn terrain2color(ty: TerrainTy) -> Color {
    match ty {
        TerrainTy::Empty => Color::rgba(0.0, 0.0, 0.0, 0.0),
        TerrainTy::Plain => Color::rgb(0.4, 0.3, 0.0),
        TerrainTy::Wall => Color::rgb(0.5, 0.1, 0.0),
        TerrainTy::Bridge => Color::rgb(0.0, 0.8, 0.0),
    }
}

fn on_new_terrain(
    mut cmd: Commands,
    mut new_terrain: EventReader<NewTerrain>,
    assets: Res<terrain_assets::TerrainRenderingAssets>,
    mut materials: ResMut<Assets<terrain_assets::TerrainMaterial>>,
    existing_tiles: Query<Entity, With<Room>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for new_terrain in new_terrain.iter() {
        info!("Poggies {:?}", new_terrain.room_id);

        for e in existing_tiles.iter() {
            // TODO: maybe update these instead of despawning all...?
            cmd.entity(e).despawn_recursive();
        }

        let mut vertices = Vec::with_capacity(new_terrain.terrain.len() * 6);
        let mut indices = Vec::with_capacity(new_terrain.terrain.len() * 6);
        let mut colors = Vec::with_capacity(new_terrain.terrain.len() * 6);
        for (p, ty) in new_terrain.terrain.iter() {
            let p = &p;
            let p = hex_axial_to_pixel(p.q as f32, p.r as f32);
            let mut p = pos_2d_to_3d(p);
            p.y -= 1.0;

            const SQRT3: f32 = 1.732_050_8;
            const W: f32 = SQRT3;
            const H: f32 = 2.0;

            let color = terrain2color(*ty);

            let vertex0ind = vertices.len() as u16;
            for offset in [
                Vec2::new(0., -H / 2.),
                Vec2::new(W / 2., -H / 4.),
                Vec2::new(W / 2., H / 4.),
                Vec2::new(0., H / 2.),
                Vec2::new(-W / 2., H / 4.),
                Vec2::new(-W / 2., -H / 4.),
            ]
            .iter()
            {
                let p = p + pos_2d_to_3d(*offset);
                vertices.push([p.x, p.y, p.z]);
                colors.push([color.r(), color.g(), color.b(), color.a()]);
            }
            // side triangle 0
            indices.push(vertex0ind + 2);
            indices.push(vertex0ind + 1);
            indices.push(vertex0ind + 0);
            // side triangle 1
            indices.push(vertex0ind + 4);
            indices.push(vertex0ind + 3);
            indices.push(vertex0ind + 2);
            // side triangle 2
            indices.push(vertex0ind + 0);
            indices.push(vertex0ind + 5);
            indices.push(vertex0ind + 4);
            // center triangle
            indices.push(vertex0ind + 4);
            indices.push(vertex0ind + 2);
            indices.push(vertex0ind + 0);
        }
        let mut mesh = Mesh::new(bevy::render::pipeline::PrimitiveTopology::TriangleList);
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.set_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.set_indices(Some(bevy::render::mesh::Indices::U16(indices)));

        let mesh = meshes.add(mesh);

        let material = materials.add(terrain_assets::TerrainMaterial {});

        cmd.spawn_bundle(MeshBundle {
            mesh,
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                assets.pipeline.clone_weak(),
            )]),
            ..Default::default()
        })
        .insert(material)
        .insert(Room);
    }
}

fn setup(
    asset_server: Res<AssetServer>,
    mut pipelines: ResMut<Assets<bevy::render::pipeline::PipelineDescriptor>>,
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

    *terrain_rendering_assets = terrain_assets::TerrainRenderingAssets {
        pipeline: pipeline_handle,
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
