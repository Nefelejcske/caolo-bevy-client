mod bots;
mod caosim;

use bevy::prelude::*;

pub struct RoomCameraTag;

fn setup(mut clear: ResMut<ClearColor>, mut cmd: Commands) {
    *clear = ClearColor(Color::rgb(0.34, 0.34, 0.34));

    // spawn the camera looking at the world
    cmd.spawn()
        .insert_bundle(PerspectiveCameraBundle::new_3d())
        .insert(
            Transform::from_translation(Vec3::new(0.0, 0.0, 100.0))
                .looking_at(caosim::hex_axial_to_pixel(25.0, 25.0).extend(0.0), Vec3::Y),
        )
        .insert(RoomCameraTag);
}

fn main() {
    App::build()
        .insert_resource(DefaultTaskPoolOptions::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(caosim::CaoSimPlugin)
        .add_plugin(bots::BotsPlugin)
        .add_startup_system(setup.system())
        .init_resource::<ClearColor>()
        .run();
}
