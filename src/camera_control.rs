use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::caosim;

pub struct CameraControlPlugin;
pub struct RoomCameraTag;
// outer entity, holding the camera
pub struct RoomCameraRigTag;

struct TargetRotation(Quat);
struct Velocity(f32);
struct DefaultPosition(Vec3);

fn rig_rotation_system(mut cam_rigs: Query<(&mut Transform, &TargetRotation)>) {
    for (mut tr, rot) in cam_rigs.iter_mut() {
        tr.rotation = tr.rotation.slerp(rot.0, 0.5);
    }
}

fn rig_input_system(
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
    for (mut tr, mut rot, v, default_pos) in cam_rigs.iter_mut() {
        let mut dtranslation = Vec3::ZERO;

        let sideways = tr.local_x();
        let forward = tr.local_z();

        let mut drotation = Quat::IDENTITY;

        for key in keyboard_input.get_pressed() {
            match key {
                // translation
                KeyCode::W => dtranslation += forward,
                KeyCode::S => dtranslation -= forward,
                KeyCode::D => dtranslation -= sideways,
                KeyCode::A => dtranslation += sideways,
                KeyCode::Space => tr.translation = default_pos.0,
                // rotation
                KeyCode::E => drotation = drotation.mul_quat(Quat::from_rotation_y(TAU / 6.0)),
                KeyCode::Q => drotation = drotation.mul_quat(Quat::from_rotation_y(TAU / -6.0)),
                _ => {}
            }
        }

        tr.translation += dtranslation.normalize_or_zero() * v.0 * time.delta_seconds();
        rot.0 = rot.0.mul_quat(drotation);
    }
}

fn setup(mut cmd: Commands) {
    let map_mid = caosim::hex_axial_to_pixel(30., 30.);
    let map_mid = Vec3::new(map_mid.x, 0.0, map_mid.y);

    let outertr = Transform::from_translation(Vec3::new(map_mid.x, 0., map_mid.z));

    let mut innertr = Transform::from_translation(Vec3::new(map_mid.x, 75., map_mid.z - 65.0));
    innertr.look_at(map_mid, Vec3::Y);
    innertr.translation.x = 0.0;
    innertr.translation.y = 75.0;
    innertr.translation.z = -75.0;

    // spawn the camera looking at the world
    cmd.spawn()
        .insert_bundle((
            Velocity(50.0),
            RoomCameraRigTag,
            outertr,
            GlobalTransform::default(),
            TargetRotation(outertr.rotation.clone()),
            DefaultPosition(outertr.translation),
        ))
        .with_children(move |c| {
            c.spawn()
                .insert_bundle(PerspectiveCameraBundle::new_3d())
                .insert(innertr)
                .insert(RoomCameraTag);
        });
}

impl Plugin for CameraControlPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system(rig_input_system.system())
            .add_system(rig_rotation_system.system());
    }
}
