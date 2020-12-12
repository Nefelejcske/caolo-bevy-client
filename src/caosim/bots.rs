use bevy::{
    prelude::*,
    render::{
        mesh::shape,
        pipeline::{DynamicBinding, PipelineDescriptor, PipelineSpecialization, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
    type_registry::TypeUuid,
};
use cao_math::{hex::axial_to_pixel_mat_pointy, vec::vec2::Vec2};
use tracing::{event, Level};

const BOT_FRAGMENT_SHADER: &str = include_str!("./bot_fragment.glsl");
const BOT_VERTEX_SHADER: &str = include_str!("./bot_vertex.glsl");

use super::{
    sim::{CaoBot, CaoEntityId},
    Bot, HexPos,
};

#[derive(Debug, Clone, Copy)]
pub struct CurrentPos(pub Vec2);

impl Default for CurrentPos {
    fn default() -> Self {
        Self(Vec2::new(0., 0.))
    }
}
#[derive(Debug, Clone, Copy)]
pub struct TargetPos(pub Vec2);

impl Default for TargetPos {
    fn default() -> Self {
        Self(Vec2::new(0., 0.))
    }
}

pub fn update_target_pos(mut q: Query<(Mut<TargetPos>, &HexPos, &Bot)>) {
    let matrix = axial_to_pixel_mat_pointy();
    for (mut tp, hex, _) in q.iter_mut() {
        let p = matrix.right_prod(Vec2::new(hex.q as f32, hex.r as f32));
        tp.0 = p;
    }
}

pub fn update_current_pos(
    time: Res<Time>,
    mut q: Query<(Mut<CurrentPos>, Mut<Transform>, &TargetPos)>,
) {
    for (mut current, mut transform, target) in q.iter_mut() {
        let diff = target.0 - current.0;

        if diff.len_sq() > 30. {
            // if too far away just teleport
            current.0 = target.0;
        } else {
            let diff = diff * time.delta_seconds * 0.5;
            current.0 += diff;
        }

        transform.translation = Vec3::new(current.0.x, current.0.y, 0.0);
    }
}

#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "262e94f6-e1b3-4d11-ae3f-e33c9e1c8c4a"]
pub struct BotMaterial {
    pub color: Color,
}

pub mod resources {
    use bevy::prelude::*;
    use bevy::render::pipeline::PipelineDescriptor;

    #[derive(Default)]
    pub struct BotRenderingAssets {
        pub pipeline: Handle<PipelineDescriptor>,
        pub mesh: Handle<Mesh>,
    }
}

pub fn setup(
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<RenderGraph>,
    mut bot_rendering_assets: ResMut<resources::BotRenderingAssets>,
) {
    // Create a new shader pipeline
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, BOT_VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(
            ShaderStage::Fragment,
            BOT_FRAGMENT_SHADER,
        ))),
    }));

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind BotMaterial resources to our shader
    render_graph.add_system_node(
        "bot_material",
        AssetRenderResourcesNode::<BotMaterial>::new(true),
    );

    // Add a Render Graph edge connecting our new "my_material" node to the main pass node. This ensures "my_material" runs before the main pass
    render_graph
        .add_node_edge("bot_material", base::node::MAIN_PASS)
        .unwrap();

    let mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));

    *bot_rendering_assets = resources::BotRenderingAssets {
        mesh,
        pipeline: pipeline_handle,
    };
}

pub fn spawn_bot(
    cmd: &mut Commands,
    cao_bot: &CaoBot,
    assets: &resources::BotRenderingAssets,
    materials: &mut Assets<BotMaterial>,
) {
    event!(Level::DEBUG, "Spawning new bot, id: {}", cao_bot.id);
    // Create a new material
    let c = cao_bot.id as f32;
    let material = materials.add(BotMaterial {
        color: Color::rgb(c.cos(), c.sin(), 0.0),
    });
    cmd.spawn((
        CaoEntityId(cao_bot.id),
        Bot,
        cao_bot.pos.room_pos,
        TargetPos::default(),
        CurrentPos::default(),
    ))
    .with_bundle(MeshComponents {
        mesh: assets.mesh.clone(),
        render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
            assets.pipeline.clone(),
            PipelineSpecialization {
                dynamic_bindings: vec![
                    // Transform
                    DynamicBinding {
                        bind_group: 1,
                        binding: 0,
                    },
                    // MyMaterial_color
                    DynamicBinding {
                        bind_group: 1,
                        binding: 1,
                    },
                ],
                ..Default::default()
            },
        )]),
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ..Default::default()
    })
    .with(material);
}
