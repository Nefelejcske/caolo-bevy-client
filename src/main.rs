mod bots;
mod camera_control;
mod caosim;
mod main_menu;
mod mining;
mod resources;
mod room_interaction;
mod room_ui;
mod terrain;

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum AppState {
    MainMenu,
    Room,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum EntityType {
    Undefined = 0,
    Bot,
    Resource,
    Structure,
}

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
        .add_plugin(main_menu::MainMenuPlugin)
        .add_plugin(bevy_egui::EguiPlugin)
        .add_plugin(room_ui::RoomUiPlugin)
        .add_plugin(room_interaction::RoomInteractionPlugin)
        .add_state(AppState::MainMenu)
        .insert_resource(ClearColor(Color::rgb(0.34, 0.34, 0.34)))
        .add_startup_system(setup.system())
        .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .run();
}
