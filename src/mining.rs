use crate::{
    bots::Bot,
    caosim::SimEntityId,
    resources::{Resource, ResourceIdMap},
};
use bevy::{
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph,
    },
};

#[derive(Debug)]
pub struct MiningEvent {
    pub bot_id: Entity,
    pub resource_id: SimEntityId,
}

struct MiningLaserAnimation(pub Timer);

mod assets {

    use bevy::prelude::*;
    use bevy::reflect::TypeUuid;
    use bevy::render::{pipeline::PipelineDescriptor, renderer::RenderResources};

    #[derive(Default)]
    pub struct MiningLaserRenderingAssets {
        pub pipeline: Handle<PipelineDescriptor>,
        pub mesh: Handle<Mesh>,
    }

    #[derive(RenderResources, Default, TypeUuid)]
    #[uuid = "59a6ac7a-651a-4c09-824e-535a4f4cfb8a"]
    pub struct MiningLaserMaterial {
        pub color: Color,
        pub t: f32,
    }
}

#[inline]
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn laser_animation_system(
    time: Res<Time>,
    mut q: Query<(
        &Handle<assets::MiningLaserMaterial>,
        &mut MiningLaserAnimation,
    )>,
    mut materials: ResMut<Assets<assets::MiningLaserMaterial>>,
) {
    let delta = time.delta();
    for (mat, mut t) in q.iter_mut() {
        t.0.tick(delta);
        let mut mat = materials
            .get_mut(mat)
            .expect("No material for this laser boi");
        mat.t = smoothstep(t.0.elapsed_secs());
    }
}

fn cleanup_system(mut cmd: Commands, q: Query<(Entity, &MiningLaserAnimation)>) {
    for (e, t) in q.iter() {
        if t.0.finished() {
            cmd.entity(e).despawn_recursive();
        }
    }
}

fn spawn_laser(
    cmd: &mut Commands,
    assets: &assets::MiningLaserRenderingAssets,
    materials: &mut Assets<assets::MiningLaserMaterial>,
    from: Vec3,
    to: Vec3,
) {
    let material = materials.add(assets::MiningLaserMaterial {
        color: Color::rgb(0.8, 0.8, 0.0),
        t: 0.0,
    });
    let mut transform = Transform::from_translation(from);
    transform.look_at(to, Vec3::Y);
    transform.translation = from.lerp(to, 0.5);
    transform.translation.y = 1.05;

    cmd.spawn_bundle((transform, GlobalTransform::default()))
        .with_children(|parent| {
            let mut transform = Transform::from_rotation(
                Quat::from_rotation_z(std::f32::consts::TAU / 4.0)
                    .mul_quat(Quat::from_rotation_y(std::f32::consts::TAU / 4.0)),
            );
            let d = (to - from).length();
            transform.scale.x = d;
            parent
                .spawn_bundle(MeshBundle {
                    mesh: assets.mesh.clone_weak(),
                    render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                        assets.pipeline.clone_weak(),
                    )]),
                    transform,
                    ..Default::default()
                })
                .insert(material)
                .insert(MiningLaserAnimation(Timer::from_seconds(0.88, false)));
        });
}

fn handle_mining(
    mut events: EventReader<MiningEvent>,
    resource_ids: Res<ResourceIdMap>,
    query_set: QuerySet<(
        Query<&GlobalTransform, With<Bot>>,
        Query<&GlobalTransform, With<Resource>>,
    )>,
    mut cmd: Commands,
    mut materials: ResMut<Assets<assets::MiningLaserMaterial>>,
    assets: Res<assets::MiningLaserRenderingAssets>,
) {
    for event in events.iter() {
        if let Some(resource_id) = (*resource_ids).0.get(&event.resource_id) {
            if let Some((resource_tr, bot_tr)) = query_set
                .q1()
                .get(*resource_id)
                .and_then(|res| query_set.q0().get(event.bot_id).map(|x| (res, x)))
                .ok()
            {
                debug!(
                    "Spawning mining laser between {:?} {:?}",
                    resource_tr.translation, bot_tr.translation
                );
                spawn_laser(
                    &mut cmd,
                    &*assets,
                    &mut *materials,
                    bot_tr.translation,
                    resource_tr.translation,
                );
            }
        } else {
            warn!("Resource does not exist {:?}", event);
        }
    }
}

fn setup(
    asset_server: Res<AssetServer>,
    mut pipelines: ResMut<Assets<bevy::render::pipeline::PipelineDescriptor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<render_graph::RenderGraph>,
    mut rendering_assets: ResMut<assets::MiningLaserRenderingAssets>,
) {
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(
        bevy::render::shader::ShaderStages {
            vertex: asset_server.load::<Shader, _>("shaders/mining_laser.vert"),
            fragment: Some(asset_server.load::<Shader, _>("shaders/mining_laser.frag")),
        },
    ));
    render_graph.add_system_node(
        "mining_laser_material",
        render_graph::AssetRenderResourcesNode::<assets::MiningLaserMaterial>::new(true),
    );
    render_graph
        .add_node_edge("mining_laser_material", render_graph::base::node::MAIN_PASS)
        .unwrap();
    let mesh = meshes.add(Mesh::from(shape::Quad {
        size: Vec2::ONE,
        flip: false,
    }));
    *rendering_assets = assets::MiningLaserRenderingAssets {
        pipeline: pipeline_handle,
        mesh,
    };
}

pub struct MiningPlugin;

impl Plugin for MiningPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system(handle_mining.system())
            .add_system(laser_animation_system.system())
            .add_system(cleanup_system.system())
            .init_resource::<assets::MiningLaserRenderingAssets>()
            .add_asset::<assets::MiningLaserMaterial>()
            .add_event::<MiningEvent>();
    }
}
