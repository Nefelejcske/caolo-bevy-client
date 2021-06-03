use bevy::prelude::*;

use crate::caosim;

pub struct CameraControlPlugin;
pub struct RoomCameraTag;
pub struct RoomCameraRigTag;

struct Velocity(f32);

fn move_camera(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut cam_rigs: Query<(&mut Transform, &Velocity), With<RoomCameraRigTag>>,
) {
    for (mut tr, v) in cam_rigs.iter_mut() {
        let mut delta = Vec3::ZERO;

        let sideways = tr.local_x();
        let forward = tr.local_z();

        for key in keyboard_input.get_pressed() {
            match key {
                KeyCode::W => delta += forward,
                KeyCode::S => delta -= forward,
                KeyCode::D => delta -= sideways,
                KeyCode::A => delta += sideways,
                KeyCode::Space => tr.translation = Vec3::ZERO,
                _ => {}
            }
        }

        tr.translation += delta.normalize_or_zero() * v.0 * time.delta_seconds();
    }
}

fn setup(mut cmd: Commands) {
    let map_mid = caosim::hex_axial_to_pixel(30., 30.);
    let map_mid = Vec3::new(map_mid.x, 0.0, map_mid.y);

    let mut innertr = Transform::from_translation(Vec3::new(map_mid.x, 75., map_mid.z - 65.0))
        .looking_at(map_mid, Vec3::Y);
    innertr.translation.z -= 10.0;

    // spawn the camera looking at the world
    cmd.spawn()
        .insert_bundle((
            Velocity(50.0),
            RoomCameraRigTag,
            Transform::default(),
            GlobalTransform::default(),
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
            .add_system(move_camera.system());
    }
}
