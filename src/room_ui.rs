use crate::caosim::{ConnectionStateRes, NewEntities, NewTerrain};
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

pub struct RoomUiPlugin;

impl Plugin for RoomUiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(Diag::default())
            .add_system_set(
                SystemSet::on_update(crate::AppState::Room).with_system(update_ui_system.system()),
            )
            .add_system(on_new_entities.system())
            .add_system(on_new_terrain.system());
    }
}
