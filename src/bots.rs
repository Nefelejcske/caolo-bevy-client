pub mod assets;

use bevy::{
    prelude::*,
    render::{pipeline::PipelineDescriptor, render_graph},
};

use crate::caosim::NewEntities;

pub struct Bot;
pub struct LastPos(pub Vec2);
pub struct NextPos(pub Vec2);
pub struct CurrentPos(pub Vec2);

#[derive(Debug, Clone, Default)]
struct WalkTimer(Timer);

pub struct BotsPlugin;

pub const STEP_TIME: f32 = 0.8;

pub fn spawn_bot(
    cmd: &mut Commands,
    pos: Vec2,
    assets: &assets::BotRenderingAssets,
    materials: &mut Assets<assets::BotMaterial>,
) -> Entity {
    let material = materials.add(assets::BotMaterial {
        color: Color::rgb(0.2, 0.8, 0.8),
        time: 0.0,
    });

    cmd.spawn_bundle(MeshBundle {
        mesh: assets.mesh.clone_weak(),
        render_pipelines: RenderPipelines::from_pipelines(vec![
            bevy::render::pipeline::RenderPipeline::new(assets.pipeline.clone_weak()),
        ]),
        ..Default::default()
    })
    .insert_bundle((Bot, LastPos(pos), NextPos(pos), CurrentPos(pos)))
    .insert(material)
    .id()
}

fn update_bot_materials(
    time: Res<Time>,
    mut materials: ResMut<Assets<assets::BotMaterial>>,
    query: Query<&Handle<assets::BotMaterial>>,
) {
    query.for_each_mut(move |handle| {
        if let Some(mat) = materials.get_mut(&*handle) {
            mat.time = time.seconds_since_startup() as f32;
        }
    });
}

fn update_transform(mut query: Query<(&CurrentPos, &mut Transform)>) {
    for (CurrentPos(p), mut tr) in query.iter_mut() {
        tr.translation = p.extend(0.0);
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn update_pos(
    mut t: ResMut<WalkTimer>,
    time: Res<Time>,
    mut query: Query<(&LastPos, &NextPos, &mut CurrentPos), With<Bot>>,
) {
    t.0.tick(time.delta());
    let WalkTimer(ref mut t) = &mut *t;
    let t = t.elapsed_secs() / STEP_TIME;
    let t = smoothstep(t);
    for (last, next, mut curr) in query.iter_mut() {
        curr.0 = last.0.lerp(next.0, t);
    }
}

fn on_new_entities(mut t: ResMut<WalkTimer>, mut new_entities: EventReader<NewEntities>) {
    if new_entities.iter().next().is_some() {
        t.0.reset();
    }
}

fn setup(
    mut t: ResMut<WalkTimer>,
    asset_server: Res<AssetServer>,
    pipelines: ResMut<Assets<bevy::render::pipeline::PipelineDescriptor>>,
    meshes: ResMut<Assets<Mesh>>,
    render_graph: ResMut<render_graph::RenderGraph>,
    bot_rendering_assets: ResMut<assets::BotRenderingAssets>,
) {
    t.0 = Timer::from_seconds(STEP_TIME, false);
    _setup_bot_rendering(
        asset_server,
        pipelines,
        meshes,
        render_graph,
        bot_rendering_assets,
    );
}

fn _setup_bot_rendering(
    asset_server: Res<AssetServer>,
    mut pipelines: ResMut<Assets<bevy::render::pipeline::PipelineDescriptor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<render_graph::RenderGraph>,
    mut bot_rendering_assets: ResMut<assets::BotRenderingAssets>,
) {
    asset_server.watch_for_changes().unwrap();

    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(
        bevy::render::shader::ShaderStages {
            vertex: asset_server.load::<Shader, _>("shaders/bot.vert"),
            fragment: Some(asset_server.load::<Shader, _>("shaders/bot.frag")),
        },
    ));

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind BotMaterial resources to our shader
    render_graph.add_system_node(
        "bot_material",
        render_graph::AssetRenderResourcesNode::<assets::BotMaterial>::new(true),
    );

    // Add a Render Graph edge connecting our new "bot_material" node to the main pass node. This ensures "bot_material" runs before the main pass
    render_graph
        .add_node_edge("bot_material", render_graph::base::node::MAIN_PASS)
        .unwrap();

    let mesh = meshes.add(Mesh::from(shape::Icosphere {
        radius: 0.67,
        subdivisions: 8,
    }));

    *bot_rendering_assets = assets::BotRenderingAssets {
        mesh,
        pipeline: pipeline_handle,
    };
}

impl Plugin for BotsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(update_pos.system())
            .add_startup_system(setup.system())
            .add_system(on_new_entities.system())
            .add_system(update_transform.system())
            .add_system(update_bot_materials.system())
            .init_resource::<assets::BotRenderingAssets>()
            .add_asset::<assets::BotMaterial>()
            .init_resource::<WalkTimer>();
    }
}
