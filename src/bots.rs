pub mod bot_assets;

use bevy::{
    ecs::system::EntityCommands,
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph,
    },
};

use crate::{
    cao_entities::{pos_2d_to_3d, EntityMetadata, EntityMovedEvent, NewEntityEvent},
    cao_sim_client::cao_sim_model::{self, WorldPosition},
    mining::MiningEvent,
    room_interaction::SelectedEntity,
    AppState,
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

pub struct BotsPlugin;

pub const STEP_TIME: f32 = 0.8;

fn build_bot(
    cmd: &mut EntityCommands,
    pos: Vec2,
    assets: &bot_assets::BotRenderingAssets,
    materials: &mut Assets<bot_assets::BotMaterial>,
) {
    let material = materials.add(bot_assets::BotMaterial {
        color: Color::rgb(0.2, 0.8, 0.8),
        time: 0.0,
        selected: 0,
    });

    let orient = Quat::default();

    cmd.insert_bundle((
        Bot,
        LastPos(pos),
        NextPos(pos),
        CurrentPos(pos),
        LastRotation(orient),
        NextRotation(orient),
        CurrentRotation(orient),
        Transform::default(),
        GlobalTransform::default(),
        WalkTimer(Timer::from_seconds(STEP_TIME, false)),
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
    });
}

fn update_bot_materials(
    time: Res<Time>,
    selected: Res<SelectedEntity>,
    mut materials: ResMut<Assets<bot_assets::BotMaterial>>,
    query: Query<(&Parent, &Handle<bot_assets::BotMaterial>)>,
) {
    query.for_each_mut(move |(entity, handle)| {
        if let Some(mat) = materials.get_mut(&*handle) {
            mat.time = time.seconds_since_startup() as f32;
            mat.selected = selected
                .entity
                .map(|id| id == **entity)
                .map(|x| x as i32)
                .unwrap_or(0);
        }
    });
}

fn update_transform_pos2d(mut query: Query<(&CurrentPos, &mut Transform)>) {
    for (CurrentPos(p), mut tr) in query.iter_mut() {
        tr.translation.x = p.x;
        tr.translation.z = p.y;
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

fn update_walkies_system(time: Res<Time>, mut q: Query<&mut WalkTimer>) {
    let delta = time.delta();
    for mut t in q.iter_mut() {
        t.0.tick(delta);
    }
}

fn update_pos_system(mut query: Query<(&LastPos, &NextPos, &mut CurrentPos, &WalkTimer)>) {
    for (last, next, mut curr, t) in query.iter_mut() {
        curr.0 = last.0.lerp(next.0, ezing::quad_inout(t.0.percent()));
    }
}

fn update_orient_system(
    mut query: Query<
        (
            &LastRotation,
            &NextRotation,
            &mut CurrentRotation,
            &WalkTimer,
        ),
        With<Bot>,
    >,
) {
    for (last, next, mut curr, t) in query.iter_mut() {
        let t = t.0.percent();
        curr.0 = last.0.slerp(next.0, ezing::quad_inout(t));
    }
}

type BotPosQuery<'a, 'b> = Query<
    'a,
    (
        &'b mut LastPos,
        &'b mut NextPos,
        &'b mut LastRotation,
        &'b mut NextRotation,
        &'b mut WalkTimer,
    ),
    With<Bot>,
>;

fn on_bot_move_system(
    mut moved_entities: EventReader<EntityMovedEvent>,
    mut bot_q: BotPosQuery,
    bot_data: Query<(&cao_sim_model::Bot, &EntityMetadata)>,
) {
    for event in moved_entities
        .iter()
        .filter(|m| m.ty == crate::cao_entities::EntityType::Bot)
    {
        let (bot, meta) = match bot_data.get(event.id) {
            Ok(b) => b,
            Err(err) => {
                error!(
                    "Received entity moved event but the entity can't be queried ({:?}): {:?}",
                    event, err
                );
                continue;
            }
        };
        update_from_to(meta.id, bot, &mut bot_q);
    }
}

fn on_new_entities_system(
    mut cmd: Commands,
    bot_assets: Res<bot_assets::BotRenderingAssets>,
    mut bot_materials: ResMut<Assets<bot_assets::BotMaterial>>,
    mut new_entities: EventReader<NewEntityEvent>,
    q_meta: Query<(&EntityMetadata, &WorldPosition)>,
) {
    for new_entity_event in new_entities
        .iter()
        .filter(|m| m.ty == crate::cao_entities::EntityType::Bot)
    {
        let (meta, pos) = q_meta.get(new_entity_event.id).unwrap();
        build_bot(
            &mut cmd.entity(meta.id),
            pos.as_pixel(),
            &*bot_assets,
            &mut *bot_materials,
        );

        // TODO mining event
        // if let Some(mine) = &bot.mine_intent {
        //     mining_event.send(MiningEvent {
        //         bot_id: meta.id,
        //         resource_id: SimEntityId(mine.target_id),
        //     });
        // }
    }
}

fn update_from_to(bot_id: Entity, bot: &cao_sim_model::Bot, bot_q: &mut BotPosQuery) {
    let (mut last_pos, mut next_pos, mut last_rot, mut next_rot, mut t) =
        match bot_q.get_mut(bot_id) {
            Ok(x) => x,
            Err(err) => {
                trace!("Failed to query bot {:?}", err);
                return;
            }
        };
    t.0.reset();

    last_pos.0 = next_pos.0;
    next_pos.0 = bot.pos.as_pixel();

    last_rot.0 = next_rot.0;
    if next_pos.0 != last_pos.0 {
        let velocity: Vec2 = (next_pos.0 - last_pos.0).normalize();
        next_rot.0 =
            Quat::from_rotation_y(-(velocity.dot(Vec2::Y).clamp(-0.999999, 0.999999)).acos());
    }
}

fn setup_system(
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
        app.add_startup_system(setup_system.system())
            .add_system_set(
                SystemSet::on_update(AppState::Room)
                    .with_system(update_pos_system.system())
                    .with_system(on_new_entities_system.system())
                    .with_system(on_bot_move_system.system())
                    .with_system(update_transform_pos2d.system())
                    .with_system(update_transform_rot.system())
                    .with_system(update_bot_materials.system())
                    .with_system(update_walkies_system.system())
                    .with_system(update_orient_system.system()),
            )
            .init_resource::<bot_assets::BotRenderingAssets>()
            .add_asset::<bot_assets::BotMaterial>();
    }
}
