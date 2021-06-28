use crate::{bots::Bot, camera_control::RoomCameraTag, caosim::SimEntityId};
use bevy::prelude::*;

#[derive(Debug)]
pub struct MiningEvent {
    pub bot_id: Entity,
    pub resource_id: SimEntityId,
}

#[derive(Debug, Clone)]
struct MiningLaserLifeTime(pub Timer);
#[derive(Debug, Copy, Clone)]
struct Icon;

mod assets {
    use bevy::{prelude::Handle, sprite::TextureAtlas};

    #[derive(Default)]
    pub struct MiningLaserRenderingAssets {
        pub atlas: Handle<TextureAtlas>,
    }
}

fn laser_animation_system(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<(&mut Timer, &mut TextureAtlasSprite, &Handle<TextureAtlas>)>,
) {
    let delta = time.delta();

    for (mut timer, mut sprite, texture_atlas_handle) in query.iter_mut() {
        timer.tick(delta);
        if timer.finished() {
            let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
            sprite.index = ((sprite.index as usize + 1) % texture_atlas.textures.len()) as u32;
        }
    }
}

fn cleanup_system(
    time: Res<Time>,
    mut cmd: Commands,
    mut q: Query<(Entity, &mut MiningLaserLifeTime)>,
) {
    let delta = time.delta();
    for (e, mut t) in q.iter_mut() {
        t.0.tick(delta);
        if t.0.finished() {
            cmd.entity(e).despawn_recursive();
        }
    }
}

fn move_icon_with_cam(
    q_cam: Query<&GlobalTransform, With<RoomCameraTag>>,
    mut q_icons: Query<&mut Transform, With<Icon>>,
) {
    let cam_tr = q_cam.iter().next().expect("No camera found");
    let cam_fw = cam_tr.local_z();
    let cam_up = cam_tr.local_y();

    for mut transform in q_icons.iter_mut() {
        // look at the camera's position mirrored to `from`
        let pos = transform.translation + cam_fw;
        transform.look_at(pos, cam_up);
    }
}

fn spawn_icon(cmd: &mut Commands, assets: &assets::MiningLaserRenderingAssets, from: Vec3) {
    let mut transform = Transform::from_translation(from);
    transform.translation.y = 1.1;

    cmd.spawn_bundle((
        transform,
        GlobalTransform::default(),
        MiningLaserLifeTime(Timer::from_seconds(0.88, false)),
    ))
    .with_children(|parent| {
        let transform = Transform::from_scale(Vec3::splat(0.05));
        parent
            .spawn_bundle(SpriteSheetBundle {
                texture_atlas: assets.atlas.clone(),
                transform,
                ..Default::default()
            })
            .insert_bundle((Timer::from_seconds(0.15, true), Icon));
    });
}

fn handle_mining(
    mut events: EventReader<MiningEvent>,
    q: Query<&GlobalTransform, With<Bot>>,
    mut cmd: Commands,
    assets: Res<assets::MiningLaserRenderingAssets>,
) {
    for event in events.iter() {
        if let Some(bot_tr) = q.get(event.bot_id).ok() {
            debug!("Spawning mining icon at {:?}", bot_tr.translation);
            spawn_icon(&mut cmd, &*assets, bot_tr.translation);
        }
    }
}

fn setup(
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut rendering_assets: ResMut<assets::MiningLaserRenderingAssets>,
) {
    let texture_handle = asset_server.load("sprites/mining.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(32.0, 32.0), 4, 1);
    let texture_atlas_handle: Handle<TextureAtlas> = texture_atlases.add(texture_atlas);
    *rendering_assets = assets::MiningLaserRenderingAssets {
        atlas: texture_atlas_handle,
    };
}

pub struct MiningPlugin;

impl Plugin for MiningPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_set(
                SystemSet::on_update(crate::AppState::Room)
                    .with_system(handle_mining.system())
                    .with_system(laser_animation_system.system())
                    .with_system(cleanup_system.system())
                    .with_system(move_icon_with_cam.system()),
            )
            .init_resource::<assets::MiningLaserRenderingAssets>()
            .add_event::<MiningEvent>();
    }
}
