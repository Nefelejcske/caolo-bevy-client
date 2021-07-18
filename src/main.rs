mod bots;
mod camera_control;
mod caolang;
mod caosim;
mod main_menu;
mod mining;
mod resources;
mod room_interaction;
mod room_ui;
mod structures;
mod terrain;

use bevy::prelude::*;

pub const API_BASE_URL: &str = "https://web-snorrwe.cloud.okteto.net";
pub const WS_BASE_URL: &str = "wss://rt-snorrwe.cloud.okteto.net";

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

fn setup_system(asset_server: Res<AssetServer>) {
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
        .add_plugin(structures::StructuresPlugin)
        .add_plugin(mining::MiningPlugin)
        .add_plugin(main_menu::MainMenuPlugin)
        .add_plugin(bevy_egui::EguiPlugin)
        .add_plugin(room_ui::RoomUiPlugin)
        .add_plugin(room_interaction::RoomInteractionPlugin)
        .add_plugin(caolang::CaoLangPlugin)
        .add_state(AppState::MainMenu)
        .insert_resource(ClearColor(Color::rgb(0.34, 0.34, 0.34)))
        .add_startup_system(setup_system.system())
        .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .run();
}
