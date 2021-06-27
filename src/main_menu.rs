use crate::{
    caosim::{ConnectionState, ConnectionStateRes},
    AppState,
};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

pub struct MainMenuPlugin;

fn update_menu_system(
    egui_ctx: Res<EguiContext>,
    mut state: ResMut<State<AppState>>,
    connection_state: Res<ConnectionStateRes>,
) {
    let connection_state = connection_state.load(std::sync::atomic::Ordering::Relaxed);
    let connected = matches!(connection_state, ConnectionState::Online);

    egui::CentralPanel::default().show(egui_ctx.ctx(), |ui| {
        ui.heading("CaoLo");

        ui.horizontal(|ui| {
            ui.label("Connection state: ");

            let pl = match connection_state {
                ConnectionState::Connecting => "Connecting...",
                ConnectionState::Online => "Online",
                ConnectionState::Closed => "Closed",
                ConnectionState::Error => "Error",
            };

            ui.label(pl);
        });

        if connected && ui.button("Let's go").clicked() {
            state.set(AppState::Room).unwrap_or_default();
        }
    });
}

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set(
            SystemSet::on_update(crate::AppState::MainMenu)
                .with_system(update_menu_system.system()),
        );
    }
}
