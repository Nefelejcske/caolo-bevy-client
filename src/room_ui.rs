use crate::{
    cao_sim_client::{ConnectionStateRes, NewEntities, NewTerrain},
    room_interaction::SelectedEntity,
};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

#[derive(Debug, Default)]
struct Diag {
    time: i64,
    num_bots: usize,
    num_structures: usize,
    num_resources: usize,
    num_tiles: usize,
}

fn on_new_terrain(mut data: ResMut<Diag>, mut new_t: EventReader<NewTerrain>) {
    for t in new_t.iter() {
        data.num_tiles = t.terrain.len();
    }
}

fn on_new_entities(mut data: ResMut<Diag>, mut new_entities: EventReader<NewEntities>) {
    for entities in new_entities.iter() {
        data.time = entities.0.time;
        data.num_bots = entities.0.bots.len();
        data.num_structures = entities.0.resources.len();
        data.num_resources = entities.0.structures.len();
    }
}

fn update_ui_system(
    data: Res<Diag>,
    egui_ctx: Res<EguiContext>,
    connection_state: Res<ConnectionStateRes>,
) {
    let connection_state = connection_state.load(std::sync::atomic::Ordering::Relaxed);
    egui::Window::new("Room diagnostics").show(egui_ctx.ctx(), |ui| {
        ui.label(format!("Tick: {}", data.time));
        ui.label(format!("# of bots: {}", data.num_bots));
        ui.label(format!("# of resources: {}", data.num_resources));
        ui.label(format!("# of structures: {}", data.num_structures));
        ui.label(format!("# of hex tiles: {}", data.num_tiles));
        ui.label(format!("Connection state: {:?}", connection_state));
    });
}

fn selected_entity_window_system(
    egui_ctx: Res<EguiContext>,
    mut selected_entity: ResMut<SelectedEntity>,
    bots: Res<crate::bots::BotPayload>,
    structures: Res<crate::structures::StructurePayload>,
) {
    egui::Window::new("Selected Entity").show(egui_ctx.ctx(), |ui| {
        if let Some(selected) = selected_entity.entity {
            ui.label(format!("EntityID: {}", selected.0 .0));

            match selected_entity.ty {
                crate::EntityType::Undefined => {
                    error!("Undefined entity type for entity {:?}", selected.0 .0);
                    ui.label("Unrecognised entity!");
                }
                crate::EntityType::Bot => {
                    match bots.0.get(&selected.0) {
                        Some(bot) => {
                            ui.label(format!("Position: {:?}", bot.pos));
                            if let Some(hp) = &bot.hp {
                                ui.label(format!("Hp: {} / {}", hp.value, hp.value_max));
                            }
                            if let Some(carry) = &bot.carry {
                                ui.label(format!("Carry: {} / {}", carry.value, carry.value_max));
                            }
                            if let Some(decay) = &bot.decay {
                                ui.label(format!(
                                    "Decay: amount: {}, interval: {}, time remaining: {}",
                                    decay.hp_amount, decay.interval, decay.time_remaining
                                ));
                            }
                            if let Some(owner) = &bot.owner {
                                ui.label(format!("Owner: {}", owner.data));
                            }

                            if let Some(say) = &bot.say {
                                ui.label(format!("Bot says: {}", say));
                            }
                        }
                        // entity has died
                        None => selected_entity.entity = None,
                    }
                }
                crate::EntityType::Structure => {
                    match structures.0.get(&selected.0) {
                        Some(ent) => {
                            ui.label(format!("Position: {:?}", ent.pos));
                            let hp = &ent.hp;
                            ui.label(format!("Hp: {} / {}", hp.value, hp.value_max));
                            ui.label(format!("Owner: {}", ent.owner.data));
                        }
                        // entity has died
                        None => selected_entity.entity = None,
                    }
                }
                crate::EntityType::Resource => todo!(),
            }
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
                            .chain(selected_entity_window_system.system()),
                    ),
            )
            .add_system(on_new_entities.system())
            .add_system(on_new_terrain.system());
    }
}
