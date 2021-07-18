use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::{pipeline::PipelineDescriptor, renderer::RenderResources};

#[derive(Default)]
pub struct StructureRenderingAssets {
    pub pipeline: Handle<PipelineDescriptor>,
    pub mesh: Handle<Mesh>,
}

#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "bd81e429-e815-479a-afe1-d1624cc79dfa"]
pub struct StructureMaterial {
    pub color: Color,
    pub time: f32,
}
