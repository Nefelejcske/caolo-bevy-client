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

fn login_via_env_system(mut login_event: EventWriter<account::StartLoginEvent>) {
    let username = std::env::var("CAO_USERNAME");
    if let Ok((username, password)) =
        username.and_then(|uname| std::env::var("CAO_PW").map(|pw| (uname, pw)))
    {
        let event = account::StartLoginEvent { username, password };
        login_event.send(event);
    } else {
        debug!("No login credentials were provided via env variables");
    }
}

fn login_system(
    mut local_event: Local<account::StartLoginEvent>,
    egui_ctx: ResMut<EguiContext>, // exclusive ownership
    mut login_event: EventWriter<account::StartLoginEvent>,
    mut state: ResMut<State<AppState>>,
    error: Res<account::LastLoginError>,
    q_login: Query<(), With<account::LoginRequestTask>>,
) {
    let has_login_request_in_flight = q_login.single().is_ok();
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

            if has_login_request_in_flight {
                ui.label("…");
            } else {
                if ui.button("Login").clicked() {
                    login_event.send(local_event.clone());
                }
            }
            if ui.button("CaoLang").clicked() {
                state.set(AppState::CaoLangEditor).unwrap_or_default();
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
        app.add_startup_system(login_via_env_system.system())
            .add_system_set(
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
