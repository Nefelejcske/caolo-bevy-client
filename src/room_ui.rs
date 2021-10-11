use crate::{
    cao_sim_client::{cao_sim_model, ConnectionStateRes, NewEntities},
    room_interaction::{HoveredTile, SelectedEntity},
    terrain::CurrentRoom,
};
use bevy::{diagnostic::Diagnostics, prelude::*};
use bevy_egui::{
    egui::{self, Ui},
    EguiContext,
};

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
) {
    let connection_state = connection_state.load(std::sync::atomic::Ordering::Relaxed);
    egui::Window::new("Room diagnostics").show(egui_ctx.ctx(), |ui| {
        ui.label(format!("Tick: {}", data.time));
        ui.label(format!("Connection state: {:?}", connection_state));
        ui.label(format!("Current room: {:?}", current_room.room_id));
        ui.label(format!("Hovered tile: {:?}", hovered.axial));
    });
}

fn show_bot(this: &cao_sim_model::Bot, ui: &mut Ui) {
    ui.columns(1, |uis| {
        let ui = &mut uis[0];

        ui.heading("Bot");
        ui.label(format!("ID: {}", this.id));
        ui.label(format!("Room: {}", this.pos.room));
        ui.label(format!("Pos: {}", this.pos.pos));
        if let Some(hp) = this.hp.as_ref() {
            ui.label(format!("Health: {}/{}", hp.value, hp.value_max));
        }
        if let Some(car) = this.carry.as_ref() {
            ui.label(format!("Carrying: {}/{}", car.value, car.value_max));
        }
        if let Some(script) = this.script.as_ref() {
            ui.label(format!("Script: {}", script.data));
        }
        if let Some(owner) = this.owner.as_ref() {
            ui.label(format!("Owner: {}", owner.data));
        }
        if let Some(mine) = this.mine_intent.as_ref() {
            ui.label(format!("Mining: {}", mine.target_id));
        }
    });
}

fn show_resource(this: &cao_sim_model::Resource, ui: &mut Ui) {
    ui.columns(1, |uis| {
        let ui = &mut uis[0];

        ui.heading("Resource");
        ui.label(format!("ID: {}", this.id));
        ui.label(format!("Room: {}", this.pos.room));
        ui.label(format!("Pos: {}", this.pos.pos));
        ui.label(format!(
            "Energy: {}/{}",
            this.resource_type.energy.value, this.resource_type.energy.value_max
        ));
    });
}

fn right_panel_system(
    egui_ctx: Res<EguiContext>,
    selected_entity: Res<SelectedEntity>,
    bot_q: Query<&cao_sim_model::Bot>,
    res_q: Query<&cao_sim_model::Resource>,
    stu_q: Query<&cao_sim_model::Structure>,
) {
    egui::SidePanel::right("selected-entity")
        .min_width(250.)
        .resizable(false)
        .show(egui_ctx.ctx(), |ui| {
            ui.heading("Selected Entity");
            if let Some(selected) = selected_entity.entity {
                if let Ok(bot) = bot_q.get(selected) {
                    show_bot(bot, ui);
                } else if let Ok(structure) = stu_q.get(selected) {
                    ui.label(format!("{:#?}", structure));
                } else if let Ok(resource) = res_q.get(selected) {
                    show_resource(resource, ui);
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
