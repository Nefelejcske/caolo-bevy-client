pub mod bot_assets;

use std::collections::HashMap;

use bevy::{
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph,
    },
};

use crate::{
    caosim::{hex_axial_to_pixel, NewEntities, SimEntityId},
    mining::MiningEvent,
};

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

// apply bot specific transformation
fn bot_hex_axial_to_pixel(q: f32, r: f32) -> Vec2 {
    let res = hex_axial_to_pixel(q, r);

    let dx = (fastrand::f32() - 0.5) * 0.2;
    let dy = (fastrand::f32() - 0.5) * 0.2;

    res + Vec2::new(dx, dy - 0.2)
}

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

fn update_transform_pos(mut query: Query<(&CurrentPos, &mut Transform)>) {
    for (CurrentPos(p), mut tr) in query.iter_mut() {
        tr.translation = pos_2d_to_3d(*p);
    }
}

fn update_transform_rot(
    mut children: Local<Vec<(Entity, Quat)>>,
    mut queries: QuerySet<(Query<(&CurrentRotation, &Children)>, Query<&mut Transform>)>,
) {
    children.clear();
    for (q, chldrn) in queries.q0().iter() {
        for child in chldrn.iter() {
            children.push((*child, q.0));
        }
    }
    for (child, q) in children.iter() {
        let mut tr = queries
            .q1_mut()
            .get_mut(*child)
            .expect("Failed to query child transform");
        tr.rotation = *q;
    }
}

#[inline]
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn update_pos(
    mut t: ResMut<WalkTimer>,
    time: Res<Time>,
    mut query: Query<(&LastPos, &NextPos, &mut CurrentPos)>,
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
    bot_assets: Res<bot_assets::BotRenderingAssets>,
    mut bot_materials: ResMut<Assets<bot_assets::BotMaterial>>,
    mut new_entities: EventReader<NewEntities>,
    mut mining_event: EventWriter<MiningEvent>,
    mut bot_q: Query<
        (
            &mut LastPos,
            &mut NextPos,
            &mut LastRotation,
            &mut NextRotation,
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
            let bot_id;
            if let Some(bid) = prev.remove(&cao_id) {
                curr.insert(cao_id, bid);
                trace!("found entity {:?}", bot.id);
                update_from_to(bid, bot, &mut bot_q);
                bot_id = bid;
            } else {
                let pos = &bot.pos;
                let new_id = spawn_bot(
                    &mut cmd,
                    bot_hex_axial_to_pixel(pos.q as f32, pos.r as f32),
                    &*bot_assets,
                    &mut *bot_materials,
                );
                bot_id = new_id;

                curr.insert(cao_id, new_id);
                trace!("new entity {:?}", bot.id);
            }
            if let Some(mine) = &bot.mine_intent {
                mining_event.send(MiningEvent {
                    bot_id,
                    resource_id: SimEntityId(mine.target_id),
                });
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
            &mut LastPos,
            &mut NextPos,
            &mut LastRotation,
            &mut NextRotation,
        ),
        With<Bot>,
    >,
) {
    let (mut last_pos, mut next_pos, mut last_rot, mut next_rot) =
        bot_q.get_mut(bot_id).expect("Failed to get bot components");

    last_pos.0 = next_pos.0;
    next_pos.0 = bot_hex_axial_to_pixel(bot.pos.q as f32, bot.pos.r as f32);

    last_rot.0 = next_rot.0;
    if next_pos.0 != last_pos.0 {
        let velocity: Vec2 = (next_pos.0 - last_pos.0).normalize();
        next_rot.0 =
            Quat::from_rotation_y(-(velocity.dot(Vec2::Y).clamp(-0.999999, 0.999999)).acos());
    }
}

fn setup(
    asset_server: Res<AssetServer>,
    mut pipelines: ResMut<Assets<bevy::render::pipeline::PipelineDescriptor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<render_graph::RenderGraph>,
    mut bot_rendering_assets: ResMut<bot_assets::BotRenderingAssets>,
) {
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(
        bevy::render::shader::ShaderStages {
            vertex: asset_server.load::<Shader, _>("shaders/bot.vert"),
            fragment: Some(asset_server.load::<Shader, _>("shaders/bot.frag")),
        },
    ));
    render_graph.add_system_node(
        "bot_material",
        render_graph::AssetRenderResourcesNode::<bot_assets::BotMaterial>::new(true),
    );
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
            .add_system(update_transform_pos.system())
            .add_system(update_transform_rot.system())
            .add_system(update_bot_materials.system())
            .add_system(update_orient.system())
            .init_resource::<bot_assets::BotRenderingAssets>()
            .init_resource::<EntityMap>()
            .add_asset::<bot_assets::BotMaterial>()
            .insert_resource(WalkTimer(Timer::from_seconds(STEP_TIME, false)));
    }
}
