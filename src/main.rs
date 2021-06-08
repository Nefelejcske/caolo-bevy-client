mod bots;
mod camera_control;
mod caosim;
mod terrain;
mod resources;

use bevy::prelude::*;

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
        .add_plugin(camera_control::CameraControlPlugin)
        .add_plugin(resources::ResourcesPlugin)
        .insert_resource(ClearColor(Color::rgb(0.34, 0.34, 0.34)))
        // .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        // .add_plugin(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .run();
}
