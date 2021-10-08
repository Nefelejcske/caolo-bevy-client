use std::f32::consts::TAU;

use bevy::{input::mouse::MouseWheel, prelude::*};

use crate::{cao_sim_client, AppState};

pub struct CameraControlPlugin;
pub struct RoomCameraTag;
// outer entity, holding the camera
pub struct RoomCameraRigTag;

struct TargetRotation(Quat);

#[derive(Debug)]
struct Zoom {
    t: f32,
    min: Vec3,
    max: Vec3,
}
struct Velocity(f32);
struct DefaultPosition(Vec3);

struct RotationCooldown {
    cooling: bool,
    t: Timer,
}

fn rig_rotation_system(mut cam_rigs: Query<(&mut Transform, &TargetRotation)>) {
    for (mut tr, rot) in cam_rigs.iter_mut() {
        tr.rotation = tr.rotation.slerp(rot.0, 0.3);
    }
}

fn inv_lerp(a: f32, b: f32, val: f32) -> f32 {
    (val - a) / (b - a)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    t * (b - a) + a
}

fn eerp(a: f32, b: f32, t: f32) -> f32 {
    2.0f32.powf(lerp(a.log2(), b.log2(), t))
}

fn update_inner_camera_pos(tr: &mut Transform, zoom: &Zoom) {
    let t = eerp(1.0, 8.0, zoom.t);
    let t = inv_lerp(1.0, 8.0, t);

    let pos = zoom.min.lerp(zoom.max, t);

    tr.translation = pos;
}

fn inner_camera_input_system(
    time: Res<Time>,
    mut mouse_input: EventReader<MouseWheel>,
    mut cams: Query<(&mut Transform, &Velocity, &mut Zoom), With<RoomCameraTag>>,
) {
    for event in mouse_input.iter() {
        for (mut tr, vel, mut zoom) in cams.iter_mut() {
            zoom.t = (zoom.t - event.y * vel.0 * time.delta_seconds()).clamp(0.0, 1.0);

            update_inner_camera_pos(&mut *tr, &*zoom);
        }
    }
}

fn rig_input_system(
    mut rot_cd: ResMut<RotationCooldown>,
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut cam_rigs: Query<
        (
            &mut Transform,
            &mut TargetRotation,
            &Velocity,
            &DefaultPosition,
        ),
        With<RoomCameraRigTag>,
    >,
) {
    rot_cd.t.tick(time.delta());
    if rot_cd.t.just_finished() {
        rot_cd.cooling = false;
    }
    for (mut tr, mut rot, v, default_pos) in cam_rigs.iter_mut() {
        let mut dtranslation = Vec3::ZERO;

        let sideways = tr.local_x();
        let forward = tr.local_z();

        let mut drot = Quat::IDENTITY;
        let mut rotated = false;

        for key in keyboard_input.get_pressed() {
            match key {
                // translation
                KeyCode::W => dtranslation += forward,
                KeyCode::S => dtranslation -= forward,
                KeyCode::D => dtranslation -= sideways,
                KeyCode::A => dtranslation += sideways,
                // reset translation
                KeyCode::Space => tr.translation = default_pos.0,
                // rotation
                KeyCode::E if !rot_cd.cooling => {
                    rotated = true;
                    drot = drot.mul_quat(Quat::from_rotation_y(TAU / 6.0))
                }
                KeyCode::Q if !rot_cd.cooling => {
                    rotated = true;
                    drot = drot.mul_quat(Quat::from_rotation_y(TAU / -6.0))
                }
                _ => {}
            }
        }

        tr.translation += dtranslation.normalize_or_zero() * v.0 * time.delta_seconds();
        if rotated && !rot_cd.cooling {
            rot_cd.t.reset();
            rot_cd.cooling = true;
            rot.0 = rot.0.mul_quat(drot);
        }
    }
}

fn setup(mut cmd: Commands) {
    // TODO:
    // maybe get from an event?
    let map_mid = cao_sim_client::hex_axial_to_pixel(186., 1116.);
    let map_mid = Vec3::new(map_mid.x, 0.0, map_mid.y);

    let outertr = Transform::from_translation(Vec3::new(map_mid.x, 0., map_mid.z));

    cmd.spawn()
        .insert_bundle((
            Velocity(150.0),
            RoomCameraRigTag,
            outertr,
            GlobalTransform::default(),
            TargetRotation(outertr.rotation.clone()),
            DefaultPosition(outertr.translation),
        ))
        .with_children(move |c| {
            let mut innertr =
                Transform::from_translation(Vec3::new(map_mid.x, 100., map_mid.z - 35.0));
            innertr.look_at(map_mid, Vec3::Y);
            innertr.translation.x = 0.0;
            innertr.translation.y = 75.0;
            innertr.translation.z = -55.0;

            let zoom = Zoom {
                t: 0.5,
                min: innertr.translation - innertr.local_z() * 50.0,
                max: innertr.translation + innertr.local_z() * 250.0,
            };

            update_inner_camera_pos(&mut innertr, &zoom);

            c.spawn()
                .insert_bundle(PerspectiveCameraBundle::new_3d())
                .insert(zoom)
                .insert(innertr)
                .insert(Velocity(2.0))
                .insert(RoomCameraTag);
        });
}

impl Plugin for CameraControlPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set(SystemSet::on_enter(AppState::Room).with_system(setup.system()))
            .add_system_set(
                SystemSet::on_update(AppState::Room)
                    .with_system(rig_input_system.system())
                    .with_system(rig_rotation_system.system())
                    .with_system(inner_camera_input_system.system()),
            )
            .insert_resource(RotationCooldown {
                t: Timer::from_seconds(0.35, false),
                cooling: false,
            });
    }
}
