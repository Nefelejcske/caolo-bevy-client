use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::{pipeline::PipelineDescriptor, renderer::RenderResources};

#[derive(Default)]
pub struct ResourceRenderingAssets {
    pub pipeline: Handle<PipelineDescriptor>,
    pub mesh: Handle<Mesh>,
}

#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "f9eb12d5-b497-43bf-9eda-78eb6b56078c"]
pub struct ResourceMaterial {
    pub color: Color,
    pub time: f32,
}
