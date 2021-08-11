use std::mem;

use bevy_egui::{
    egui::{self, Id, Response},
    EguiContext,
};
use cao_lang::compiler::Lane;

use super::{card_widget, drag_src, drop_target, CurrentCompileError, LaneIndex, LaneNames};

pub fn lane_ui(
    lane: &mut Lane,
    lane_index: usize,
    lane_names: &LaneNames,
    egui_ctx: &mut EguiContext,
    src_col_row: &mut Option<(LaneIndex, usize)>,
    dst_col_row: &mut Option<(LaneIndex, usize)>,
    dropped: &mut bool,
    compile_error: &CurrentCompileError,
) -> Option<Response> {
    let mut name = lane.name.as_mut().map(|x| mem::take(x)).unwrap_or_default();
    let has_lane_error = compile_error
        .0
        .as_ref()
        .and_then(|x| x.loc.as_ref())
        .map(|x| match x.0 {
            cao_lang::compiler::LaneNode::LaneName(_) => todo!(),
            cao_lang::compiler::LaneNode::LaneId(x) => x == lane_index,
        })
        .unwrap_or(false);
    let lane_index = LaneIndex::LaneId(lane_index);
    let response = egui::Window::new(name.as_str())
        .scroll(true)
        .id(egui::Id::new("cao-lang-lane").with(lane_index))
        .show(egui_ctx.ctx(), |ui| {
            ui.columns(1, |uis| {
                let mut dst_row = 0;
                let ui = &mut uis[0];
                let resp = drop_target(
                    ui,
                    true,
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Name: ");
                            if ui.text_edit_singleline(&mut name).changed() {
                                lane.name = Some(Default::default());
                            }
                        });

                        for (card_index, card) in lane.cards.iter_mut().enumerate() {
                            let mut is_this_errored = false;
                            if let Some(loc) = compile_error.0.as_ref().and_then(|x| x.loc.as_ref())
                            {
                                is_this_errored = has_lane_error && card_index as i32 == loc.1;
                            }

                            let id = Id::new("cao-lang-item").with(lane_index).with(card_index);
                            drag_src(ui, id, |ui| {
                                let response = card_widget::card_ui(
                                    ui,
                                    card,
                                    lane_names,
                                    is_this_errored.then(|| {
                                        compile_error
                                            .0
                                            .as_ref()
                                            .map(|x| x.payload.to_string())
                                            .unwrap()
                                    }),
                                );
                                if response.hovered() {
                                    dst_row = card_index
                                }
                            });

                            if ui.memory().is_being_dragged(id) {
                                *src_col_row = Some((lane_index, card_index));
                            }
                        }
                    },
                    has_lane_error.then(|| {
                        let style = ui.visuals().widgets.active;
                        let mut stroke = style.bg_stroke;
                        stroke.color = egui::Color32::RED;
                        stroke
                    }),
                )
                .response;

                if ui.input().pointer.any_released() {
                    *dropped = true;
                    ui.input().pointer.interact_pos();
                }
                *dropped = *dropped || ui.input().pointer.any_released();
                if resp.hovered() {
                    *dst_col_row = Some((lane_index, dst_row));
                }
            });
        });
    if lane.name.is_some() {
        // restore the lane name
        lane.name = Some(name);
    }

    response
}
