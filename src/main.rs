mod bots;
mod camera_control;
mod caosim;
mod mining;
mod resources;
mod terrain;

use bevy::prelude::*;

fn setup(asset_server: Res<AssetServer>) {
    asset_server.watch_for_changes().unwrap();
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
        .add_plugin(camera_control::CameraControlPlugin)
        .add_plugin(resources::ResourcesPlugin)
        .add_plugin(mining::MiningPlugin)
        .insert_resource(ClearColor(Color::rgb(0.34, 0.34, 0.34)))
        .add_startup_system(setup.system())
        // .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        // .add_plugin(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .run();
}
