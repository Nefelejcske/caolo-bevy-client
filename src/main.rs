mod bots;
mod caosim;
mod terrain;

use bevy::prelude::*;

pub struct RoomCameraTag;

fn setup(mut clear: ResMut<ClearColor>, mut cmd: Commands) {
    *clear = ClearColor(Color::rgb(0.34, 0.34, 0.34));

    let map_mid = caosim::hex_axial_to_pixel(25., 25.);
    let map_mid = Vec3::new(map_mid.x, 0.0, map_mid.y);

    // spawn the camera looking at the world
    cmd.spawn()
        .insert_bundle(PerspectiveCameraBundle::new_3d())
        .insert(Transform::from_translation(Vec3::new(0.0, 75.0, 0.0)).looking_at(map_mid, Vec3::Y))
        .insert(RoomCameraTag);
}

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Caolo".to_string(),
            ..Default::default()
        })
        .insert_resource(DefaultTaskPoolOptions::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(caosim::CaoSimPlugin)
        .add_plugin(bots::BotsPlugin)
        .add_plugin(terrain::TerrainPlugin)
        .add_startup_system(setup.system())
        .init_resource::<ClearColor>()
        // .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        // .add_plugin(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .run();
}
