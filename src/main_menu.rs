use crate::{
    account,
    cao_sim_client::{ConnectionState, ConnectionStateRes},
    AppState,
};
use bevy::{ecs::schedule::ShouldRun, prelude::*};
use bevy_egui::{egui, EguiContext};

pub struct MainMenuPlugin;

fn is_not_logged_in_system(
    state: Res<State<AppState>>,
    token: Res<account::CurrentAuthToken>,
) -> ShouldRun {
    if matches!(state.current(), AppState::MainMenu) && token.0.is_none() {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

fn is_logged_in_system(
    state: Res<State<AppState>>,
    token: Res<account::CurrentAuthToken>,
) -> ShouldRun {
    if matches!(state.current(), AppState::MainMenu) && token.0.is_some() {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

fn login_system(
    mut local_event: Local<account::StartLoginEvent>,
    egui_ctx: ResMut<EguiContext>, // exclusive ownership
    mut login_event: EventWriter<account::StartLoginEvent>,
    error: Res<account::LastLoginError>,
) {
    egui::CentralPanel::default().show(egui_ctx.ctx(), |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("Login");

            if let Some(ref error) = error.0 {
                ui.colored_label(egui::color::Rgba::RED, error);
            }

            ui.horizontal(|ui| {
                ui.label("username");
                ui.text_edit_singleline(&mut local_event.username);
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("password");
                ui.add(egui::TextEdit::singleline(&mut local_event.password).password(true));
            });

            if ui.button("Login").clicked() {
                login_event.send(local_event.clone());
            }
        });
    });
}

fn update_menu_system(
    egui_ctx: ResMut<EguiContext>, // exclusive ownership
    mut state: ResMut<State<AppState>>,
    connection_state: Res<ConnectionStateRes>,
) {
    let connection_state = connection_state.load(std::sync::atomic::Ordering::Relaxed);
    let connected = matches!(connection_state, ConnectionState::Online);

    egui::CentralPanel::default().show(egui_ctx.ctx(), |ui| {
        ui.vertical_centered(|ui| {
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

            if connected {
                if ui.button("Let's go").clicked() {
                    state.set(AppState::Room).unwrap_or_default();
                }
                if ui.button("CaoLang").clicked() {
                    state.set(AppState::CaoLangEditor).unwrap_or_default();
                }
            }
        });
    });
}

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set(
            SystemSet::new()
                .with_system(update_menu_system.system())
                .with_run_criteria(is_logged_in_system.system()),
        )
        .add_system_set(
            SystemSet::new()
                .with_system(login_system.system())
                .with_run_criteria(is_not_logged_in_system.system()),
        );
    }
}
