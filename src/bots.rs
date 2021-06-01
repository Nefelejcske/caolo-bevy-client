pub mod bot_assets;

use std::collections::HashMap;

use bevy::{
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph,
    },
};

use crate::caosim::{hex_axial_to_pixel, NewEntities, SimEntityId};

pub struct Bot;
pub struct LastPos(pub Vec2);
pub struct NextPos(pub Vec2);
pub struct CurrentPos(pub Vec2);

pub struct LastRotation(pub Quat);
pub struct NextRotation(pub Quat);
pub struct CurrentRotation(pub Quat);

#[derive(Debug, Clone, Default)]
struct WalkTimer(Timer);

#[derive(Default)]
struct EntityMap(pub HashMap<SimEntityId, Entity>);

pub struct BotsPlugin;

pub const STEP_TIME: f32 = 0.8;

fn spawn_bot(
    cmd: &mut Commands,
    pos: Vec2,
    assets: &bot_assets::BotRenderingAssets,
    materials: &mut Assets<bot_assets::BotMaterial>,
) -> Entity {
    let material = materials.add(bot_assets::BotMaterial {
        color: Color::rgb(0.2, 0.8, 0.8),
        time: 0.0,
    });

    let orient = Quat::default();

    cmd.spawn_bundle((
        Bot,
        LastPos(pos),
        NextPos(pos),
        CurrentPos(pos),
        LastRotation(orient),
        NextRotation(orient),
        CurrentRotation(orient),
        Transform::default(),
        GlobalTransform::default(),
    ))
    .with_children(|c| {
        c.spawn_bundle(MeshBundle {
            mesh: assets.mesh.clone_weak(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                assets.pipeline.clone_weak(),
            )]),
            transform: Transform::default(),
            ..Default::default()
        })
        .insert(material);
    })
    .id()
}

fn update_bot_materials(
    time: Res<Time>,
    mut materials: ResMut<Assets<bot_assets::BotMaterial>>,
    query: Query<&Handle<bot_assets::BotMaterial>>,
) {
    query.for_each_mut(move |handle| {
        if let Some(mat) = materials.get_mut(&*handle) {
            mat.time = time.seconds_since_startup() as f32;
        }
    });
}

#[inline]
pub fn pos_2d_to_3d(p: Vec2) -> Vec3 {
    Vec3::new(p.x, 0.0, p.y)
}

fn update_transform(mut query: Query<(&CurrentPos, &CurrentRotation, &mut Transform)>) {
    for (CurrentPos(p), CurrentRotation(q), mut tr) in query.iter_mut() {
        tr.translation = pos_2d_to_3d(*p);
        tr.rotation = *q;
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

fn update_orient(
    mut t: ResMut<WalkTimer>,
    mut query: Query<(&LastRotation, &NextRotation, &mut CurrentRotation), With<Bot>>,
) {
    let WalkTimer(ref mut t) = &mut *t;
    let t = t.elapsed_secs() / STEP_TIME;
    for (last, next, mut curr) in query.iter_mut() {
        curr.0 = last.0.slerp(next.0, t);
    }
}

fn on_new_entities(
    mut cmd: Commands,
    mut walk_timer: ResMut<WalkTimer>,
    mut map: ResMut<EntityMap>,
    bot_assets: Res<crate::bots::bot_assets::BotRenderingAssets>,
    mut bot_materials: ResMut<Assets<crate::bots::bot_assets::BotMaterial>>,
    mut new_entities: EventReader<NewEntities>,
    mut bot_q: Query<
        (
            &mut crate::bots::LastPos,
            &mut crate::bots::NextPos,
            &mut crate::bots::LastRotation,
            &mut crate::bots::NextRotation,
        ),
        With<Bot>,
    >,
) {
    for new_entities in new_entities.iter() {
        walk_timer.0.reset();
        let len = map.0.len();
        let mut prev = std::mem::replace(&mut map.0, HashMap::with_capacity(len));
        let curr = &mut map.0;
        curr.clear();
        for bot in new_entities.0.bots.iter() {
            let cao_id = SimEntityId(bot.id);
            if let Some(bot_id) = prev.remove(&cao_id) {
                curr.insert(cao_id, bot_id);
                trace!("found entity {:?}", bot.id);
                update_from_to(bot_id, bot, &mut bot_q);
            } else {
                let pos = &bot.pos;
                let new_id = spawn_bot(
                    &mut cmd,
                    hex_axial_to_pixel(pos.q as f32, pos.r as f32),
                    &*bot_assets,
                    &mut *bot_materials,
                );

                curr.insert(cao_id, new_id);
                trace!("new entity {:?}", bot.id);
            }
        }
        // these entities were not sent in the current tick
        for (_, dead_entity) in prev {
            cmd.entity(dead_entity).despawn_recursive();
        }
    }
}

fn update_from_to(
    bot_id: Entity,
    bot: &crate::caosim::cao_sim_model::Bot,
    bot_q: &mut Query<
        (
            &mut crate::bots::LastPos,
            &mut crate::bots::NextPos,
            &mut crate::bots::LastRotation,
            &mut crate::bots::NextRotation,
        ),
        With<Bot>,
    >,
) {
    let (mut last_pos, mut next_pos, mut last_rot, mut next_rot) =
        bot_q.get_mut(bot_id).expect("Failed to get bot components");

    last_pos.0 = next_pos.0;
    next_pos.0 = hex_axial_to_pixel(bot.pos.q as f32, bot.pos.r as f32);

    last_rot.0 = next_rot.0;
    if next_pos.0 != last_pos.0 {
        let velocity: Vec2 = (next_pos.0 - last_pos.0).normalize();
        next_rot.0 =
            Quat::from_rotation_y(-(velocity.dot(Vec2::Y).clamp(-0.999999, 0.999999)).acos());
    }
}

fn setup(
    mut t: ResMut<WalkTimer>,
    asset_server: Res<AssetServer>,
    pipelines: ResMut<Assets<bevy::render::pipeline::PipelineDescriptor>>,
    meshes: ResMut<Assets<Mesh>>,
    render_graph: ResMut<render_graph::RenderGraph>,
    bot_rendering_assets: ResMut<bot_assets::BotRenderingAssets>,
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
    mut bot_rendering_assets: ResMut<bot_assets::BotRenderingAssets>,
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
        render_graph::AssetRenderResourcesNode::<bot_assets::BotMaterial>::new(true),
    );

    // Add a Render Graph edge connecting our new "bot_material" node to the main pass node. This ensures "bot_material" runs before the main pass
    render_graph
        .add_node_edge("bot_material", render_graph::base::node::MAIN_PASS)
        .unwrap();

    let mesh = meshes.add(Mesh::from(shape::Cube { size: 0.87 }));
    *bot_rendering_assets = bot_assets::BotRenderingAssets {
        pipeline: pipeline_handle,
        mesh,
    };
}

impl Plugin for BotsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(update_pos.system())
            .add_startup_system(setup.system())
            .add_system(on_new_entities.system())
            .add_system(update_transform.system())
            .add_system(update_bot_materials.system())
            .add_system(update_orient.system())
            .init_resource::<bot_assets::BotRenderingAssets>()
            .init_resource::<EntityMap>()
            .add_asset::<bot_assets::BotMaterial>()
            .init_resource::<WalkTimer>();
    }
}
