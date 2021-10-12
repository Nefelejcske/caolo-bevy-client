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
    ui.heading("Bot");
    ui.end_row();
    ui.label("ID");
    ui.label(this.id.to_string());
    ui.end_row();
    ui.label("Room");
    ui.label(this.pos.room.to_string());
    ui.end_row();
    ui.label("Pos");
    ui.label(this.pos.pos.to_string());
    ui.end_row();
    if let Some(hp) = this.hp.as_ref() {
        ui.label("Health");
        ui.label(format!("{}/{}", hp.value, hp.value_max));
        ui.end_row();
    }
    if let Some(car) = this.carry.as_ref() {
        ui.label("Carrying");
        ui.label(format!("{}/{}", car.value, car.value_max));
        ui.end_row();
    }
    if let Some(script) = this.script.as_ref() {
        ui.label("Script");
        ui.label(script.data.as_str());
        ui.end_row();
    }
    if let Some(owner) = this.owner.as_ref() {
        ui.label("Owner");
        ui.label(owner.data.as_str());
        ui.end_row();
    }
    if let Some(mine) = this.mine_intent.as_ref() {
        ui.label("Mining");
        ui.label(mine.target_id.to_string());
        ui.end_row();
    }
    if let Some(decay) = this.decay.as_ref() {
        ui.label("Decay");
        egui::Grid::new("current_bot_decay").show(ui, |ui| {
            ui.label("Decay Amount");
            ui.label(decay.hp_amount.to_string());
            ui.end_row();
            ui.label("Time remaining");
            ui.label(format!("{}/{}", decay.time_remaining, decay.interval));
            ui.end_row();
        });
        ui.end_row();
    }
}

fn show_structure(this: &cao_sim_model::Structure, ui: &mut Ui) {
    ui.heading("Structure");
    ui.end_row();
    ui.label("ID");
    ui.label(format!("{}", this.id));
    ui.end_row();
    ui.label("Room");
    ui.label(format!("{}", this.pos.room));
    ui.end_row();
    ui.label("Pos");
    ui.label(format!("{}", this.pos.pos));
    ui.end_row();
    ui.label("Health");
    ui.label(format!("{}/{}", this.hp.value, this.hp.value_max));
    ui.end_row();
    if let Some(owner) = this.owner.as_ref() {
        ui.label("Owner");
        ui.label(format!("{}", owner.data));
        ui.end_row();
    }
    match &this.structure_type {
        cao_sim_model::StructureType::Spawn(s) => {
            if s.time_to_spawn > 0 {
                ui.label("Time to spawn");
                ui.label(format!("{}", s.time_to_spawn));
                ui.end_row();
                ui.label("Spawning");
                ui.label(format!("{}", s.spawning));
                ui.end_row();
            }
            ui.label("Spawn queue");
            ui.label(format!(
                "[{}]",
                s.spawn_queue
                    .iter()
                    .map(|x| format!("{}", x))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            ui.end_row();
        }
    }
    if let Some(energy) = &this.energy {
        ui.label("Energy");
        ui.label(format!("{}/{}", energy.value, energy.value_max));
        ui.end_row();
    }
    if let Some(energy) = &this.energy_regen {
        ui.label("Energy Regen");
        ui.label(format!("{}", energy));
        ui.end_row();
    }
}

fn show_resource(this: &cao_sim_model::Resource, ui: &mut Ui) {
    ui.heading("Resource");
    ui.end_row();
    ui.label("ID");
    ui.label(format!("{}", this.id));
    ui.end_row();
    ui.label("Room");
    ui.label(format!("{}", this.pos.room));
    ui.end_row();
    ui.label("Pos");
    ui.label(format!("{}", this.pos.pos));
    ui.end_row();
    ui.label("Energy");
    ui.label(format!(
        "{}/{}",
        this.resource_type.energy.value, this.resource_type.energy.value_max
    ));
    ui.end_row();
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
                ui.columns(1, |uis| {
                    let ui = &mut uis[0];

                    egui::Grid::new("current_structure")
                        .striped(true)
                        .show(ui, |ui| {
                            if let Ok(bot) = bot_q.get(selected) {
                                show_bot(bot, ui);
                            } else if let Ok(structure) = stu_q.get(selected) {
                                show_structure(structure, ui);
                            } else if let Ok(resource) = res_q.get(selected) {
                                show_resource(resource, ui);
                            }
                        });
                });
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
