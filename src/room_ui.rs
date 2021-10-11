use crate::{
    cao_sim_client::{ConnectionStateRes, NewEntities},
    room_interaction::{HoveredTile, LookAtRoom, SelectedEntity},
    terrain::CurrentRoom,
};
use bevy::{diagnostic::Diagnostics, prelude::*};
use bevy_egui::{egui, EguiContext};

#[derive(Debug, Default)]
struct Diag {
    time: i64,
}

fn on_new_entities(mut data: ResMut<Diag>, mut new_entities: EventReader<NewEntities>) {
    for entities in new_entities.iter() {
        data.time = data.time.max(entities.0.time);
    }
}

fn update_ui_system(
    data: Res<Diag>,
    egui_ctx: Res<EguiContext>,
    connection_state: Res<ConnectionStateRes>,
    current_room: Res<CurrentRoom>,
    hovered: Res<HoveredTile>,
    lat_room: Res<LookAtRoom>,
) {
    let connection_state = connection_state.load(std::sync::atomic::Ordering::Relaxed);
    egui::Window::new("Room diagnostics").show(egui_ctx.ctx(), |ui| {
        ui.label(format!("Tick: {}", data.time));
        ui.label(format!("Connection state: {:?}", connection_state));
        ui.label(format!("Current room: {:?}", current_room.room_id));
        ui.label(format!("Hovered tile: {:?}", hovered.axial));
        ui.label(format!("Look at room: {:?}", lat_room.id));
    });
}

fn right_panel_system(
    egui_ctx: Res<EguiContext>,
    selected_entity: Res<SelectedEntity>,
    bot_q: Query<&crate::cao_sim_client::cao_sim_model::Bot>,
    res_q: Query<&crate::cao_sim_client::cao_sim_model::Resource>,
    stu_q: Query<&crate::cao_sim_client::cao_sim_model::Structure>,
) {
    egui::SidePanel::right("selected-entity")
        .min_width(250.)
        .show(egui_ctx.ctx(), |ui| {
            ui.heading("Selected Entity");
            if let Some(selected) = selected_entity.entity {
                if let Ok(bot) = bot_q.get(selected) {
                    ui.label(format!("{:#?}", bot));
                } else if let Ok(structure) = stu_q.get(selected) {
                    ui.label(format!("{:#?}", structure));
                } else if let Ok(resource) = res_q.get(selected) {
                    ui.label(format!("{:#?}", resource));
                }
            }
        });
}

fn diagnostics_ui_system(egui_ctx: Res<EguiContext>, diagnostics: Res<Diagnostics>) {
    egui::Window::new("Bevy diagnostics").show(egui_ctx.ctx(), |ui| {
        for diag in diagnostics.iter() {
            ui.label(format!(
                "{}: {:.5}",
                diag.name.as_ref(),
                diag.value().unwrap_or_default()
            ));
        }
    });
}

pub struct RoomUiPlugin;

impl Plugin for RoomUiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(Diag::default())
            .add_system_set(
                SystemSet::on_update(crate::AppState::Room)
                    // ui systems have to be chained
                    .with_system(
                        update_ui_system
                            .system()
                            .chain(right_panel_system.system())
                            .chain(diagnostics_ui_system.system()),
                    ),
            )
            .add_system(on_new_entities.system());
    }
}
