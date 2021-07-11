use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::{pipeline::PipelineDescriptor, renderer::RenderResources};

#[derive(Default)]
pub struct BotRenderingAssets {
    pub pipeline: Handle<PipelineDescriptor>,
    pub mesh: Handle<Mesh>,
}

#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "412bddf4-7931-42dd-9d1d-ee654d8c0d22"]
pub struct BotMaterial {
    pub color: Color,
    pub time: f32,
    pub selected: i32,
}
