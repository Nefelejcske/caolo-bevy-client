use bevy::{
    prelude::*,
    render::{
        camera::Camera,
        mesh::shape,
        pipeline::{DynamicBinding, PipelineDescriptor, PipelineSpecialization, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
    tasks::IoTaskPool,
    type_registry::TypeUuid,
};
use std::sync::{Arc, Mutex};
use tracing::{event, Level};

use super::HexPos;

const TERRAIN_FRAGMENT_SHADER: &str = include_str!("./terrain_fragment.glsl");
const TERRAIN_VERTEX_SHADER: &str = include_str!("./terrain_vertex.glsl");

#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "903083ec-b8e0-4b53-b65b-417afc488cd9"]
pub struct TerrainMaterial {
    pub color: Color,
}

pub struct SetTerrain(pub Arc<Mutex<Option<CaoWorldTerrain>>>);

#[derive(Debug, Clone, Default)]
pub struct CurrentTerrain(pub CaoWorldTerrain);
pub struct NewTerrainState;

#[derive(serde::Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum Tile {
    Plain = 0,
    Bridge = 1,
    Wall = 2,
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct CaoWorldTerrain(Vec<(HexPos, Tile)>);

pub fn fetch_terrain(pool: Res<IoTaskPool>, setter: Res<SetTerrain>) {
    event!(Level::DEBUG, "Fetching terrain");
    let setter = Arc::clone(&setter.0);
    pool.spawn(async move {
        let response: CaoWorldTerrain = surf::get("https://caolo.herokuapp.com/terrain?q=15&r=18")
            .recv_json()
            .await
            .expect("Failed to get");
        let mut setter = setter.lock().unwrap();
        *setter = Some(response);
    })
    .detach();
}

/// This system polls the SetTerrain resource for a new state and sets the state when appropriate
pub fn set_terrain(
    new_terrain: Res<SetTerrain>,
    mut current: ResMut<CurrentTerrain>,
    mut new_terrain_event: ResMut<Events<NewTerrainState>>,
    mut cams: Query<With<crate::RoomCameraTag, (&Camera, &mut Transform)>>,
) {
    if let Some(terrain) = new_terrain.0.lock().unwrap().take() {
        use cao_math::{hex::axial_to_pixel_mat_pointy, vec::vec2::Vec2};

        event!(Level::DEBUG, "Setting terrain");
        event!(Level::TRACE, "{:?}", terrain);

        let mut mid = terrain.0.iter().map(|(h, _)| *h).fold(
            Vec2::new(0., 0.),
            |mut res, HexPos { q, r }| {
                res.x += q as f32;
                res.y += r as f32;
                res
            },
        );

        mid /= terrain.0.len() as f32;
        let mid = axial_to_pixel_mat_pointy().right_prod(mid);

        for (_cam, mut transform) in cams.iter_mut() {
            transform.translation[0] = mid.x + 75.0;
            transform.translation[1] = mid.y + 25.0;
        }

        current.0 = terrain;
        new_terrain_event.send(NewTerrainState);
    }
}

pub mod resources {
    use bevy::prelude::*;
    use bevy::render::pipeline::PipelineDescriptor;

    #[derive(Default)]
    pub struct TileRenderingAssets {
        pub pipeline: Handle<PipelineDescriptor>,
        pub mesh: Handle<Mesh>,
    }
}

pub fn setup(
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<RenderGraph>,
    mut tile_rendering_assets: ResMut<resources::TileRenderingAssets>,
) {
    // Create a new shader pipeline
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(
            ShaderStage::Vertex,
            TERRAIN_VERTEX_SHADER,
        )),
        fragment: Some(shaders.add(Shader::from_glsl(
            ShaderStage::Fragment,
            TERRAIN_FRAGMENT_SHADER,
        ))),
    }));

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind BotMaterial resources to our shader
    render_graph.add_system_node(
        "terrain_material",
        AssetRenderResourcesNode::<TerrainMaterial>::new(true),
    );

    // Add a Render Graph edge connecting our new "my_material" node to the main pass node. This ensures "my_material" runs before the main pass
    render_graph
        .add_node_edge("terrain_material", base::node::MAIN_PASS)
        .unwrap();

    // TODO: not cube pls
    let mesh = meshes.add(Mesh::from(shape::Cube {
        size: 3.0f32.sqrt() / 4.,
    }));

    *tile_rendering_assets = resources::TileRenderingAssets {
        mesh,
        pipeline: pipeline_handle,
    };
}
