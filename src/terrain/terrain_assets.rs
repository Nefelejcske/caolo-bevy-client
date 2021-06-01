use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::{pipeline::PipelineDescriptor, renderer::RenderResources};

#[derive(Default)]
pub struct TerrainRenderingAssets {
    pub pipeline: Handle<PipelineDescriptor>,
    pub mesh: Handle<Mesh>,
}

#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "b510ae5d-dee1-49c8-b206-af81f36def97"]
pub struct TerrainMaterial {
    pub color: Color,
}
