mod card_ui;

use std::mem;

use bevy::prelude::*;
use bevy_egui::{
    egui::{self, color, CursorIcon, Id, InnerResponse, LayerId, Order, Sense, Shape, Ui},
    EguiContext,
};
use cao_lang::compiler::{CaoIr, Card};

pub struct CurrentProgram(pub CaoIr);

#[derive(Debug, Clone, Copy)]
pub enum LaneIndex {
    LaneId(usize),
    SchemaLane,
}

#[derive(Debug, Clone, Copy)]
pub struct OnCardDrop {
    src_lane: LaneIndex,
    dst_lane: LaneIndex,
    src_card: usize,
    dst_card: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct OnCardRemove {
    src_lane: LaneIndex,
    src_card: usize,
}

pub struct CaoLangEditorPlugin;

fn drag_src<R>(ui: &mut Ui, id: Id, body: impl FnOnce(&mut Ui) -> R) {
    let is_being_dragged = ui.memory().is_being_dragged(id);

    if !is_being_dragged {
        let response = ui.scope(body).response;

        // Check for drags:
        let response = ui.interact(response.rect, id, Sense::drag());
        if response.hovered() {
            ui.output().cursor_icon = CursorIcon::Grab;
        }
    } else {
        ui.output().cursor_icon = CursorIcon::Grabbing;

        // Paint the body to a new layer:
        let layer_id = LayerId::new(Order::Tooltip, id);
        let response = ui.with_layer_id(layer_id, body).response;

        // Now we move the visuals of the body to where the mouse is.
        // Normally you need to decide a location for a widget first,
        // because otherwise that widget cannot interact with the mouse.
        // However, a dragged component cannot be interacted with anyway
        // (anything with `Order::Tooltip` always gets an empty `Response`)
        // So this is fine!

        if let Some(pointer_pos) = ui.input().pointer.interact_pos() {
            let delta = pointer_pos - response.rect.center();
            ui.ctx().translate_layer(layer_id, delta);
        }
    }
}

fn drop_target<R>(
    ui: &mut Ui,
    can_accept_what_is_being_dragged: bool,
    body: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let margin = egui::Vec2::splat(4.0);
    let outer_rect_bounds = ui.available_rect_before_wrap();
    let inner_rect = outer_rect_bounds.shrink2(margin);
    let where_to_put_background = ui.painter().add(Shape::Noop);
    let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
    let ret = body(&mut content_ui);
    let outer_rect =
        egui::Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
    let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());

    let is_being_dragged = ui.memory().is_anything_being_dragged();
    let style = if is_being_dragged && response.hovered() && can_accept_what_is_being_dragged {
        ui.visuals().widgets.active
    } else {
        ui.visuals().widgets.inactive
    };

    let mut fill = style.bg_fill;
    let mut stroke = style.bg_stroke;
    if is_being_dragged && !can_accept_what_is_being_dragged {
        // gray out:
        fill = color::tint_color_towards(fill, ui.visuals().window_fill());
        stroke.color = color::tint_color_towards(stroke.color, ui.visuals().window_fill());
    }

    ui.painter().set(
        where_to_put_background,
        Shape::Rect {
            corner_radius: style.corner_radius,
            fill,
            stroke,
            rect,
        },
    );

    InnerResponse::new(ret, response)
}

fn on_card_remove_system(mut ir: ResMut<CurrentProgram>, mut on_drop: EventReader<OnCardRemove>) {
    let lanes = &mut ir.0.lanes;
    for OnCardRemove { src_lane, src_card } in on_drop.iter().copied() {
        match src_lane {
            LaneIndex::LaneId(id) => {
                lanes[id].cards.remove(src_card);
            }
            LaneIndex::SchemaLane => { /*noop*/ }
        };
    }
}

fn on_card_drop_system(mut ir: ResMut<CurrentProgram>, mut on_drop: EventReader<OnCardDrop>) {
    let lanes = &mut ir.0.lanes;
    for OnCardDrop {
        src_lane,
        dst_lane,
        src_card,
        dst_card: _,
    } in on_drop.iter().copied()
    {
        let card: Card = match src_lane {
            LaneIndex::LaneId(id) => lanes[id].cards.remove(src_card),
            LaneIndex::SchemaLane => todo!(),
        };

        match dst_lane {
            LaneIndex::LaneId(id) => {
                lanes[id].cards.push(card);
            }
            LaneIndex::SchemaLane => { /*noop*/ }
        }
    }
}

fn editor_ui_system(
    egui_ctx: ResMut<EguiContext>, // exclusive ownership
    mut ir: ResMut<CurrentProgram>,
    mut on_drop: EventWriter<OnCardDrop>,
    mut on_remove: EventWriter<OnCardRemove>,
) {
    let mut src_col_row = None;
    let mut dst_col_row = None;
    let mut dropped = false;
    for (lane_index, lane) in ir.0.lanes.iter_mut().enumerate() {
        let mut name = lane.name.as_mut().map(|x| mem::take(x)).unwrap_or_default();
        egui::Window::new(name.as_str())
            .scroll(false)
            .resizable(false)
            .id(egui::Id::new("cao-lang-lane").with(lane_index))
            .show(egui_ctx.ctx(), |ui| {
                ui.columns(1, |uis| {
                    let ui = &mut uis[0];
                    let resp = drop_target(ui, true, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Name: ");
                            if ui.text_edit_singleline(&mut name).changed() {
                                lane.name = Some(Default::default());
                            }
                        });

                        for (card_index, card) in lane.cards.iter_mut().enumerate() {
                            let id = Id::new("cao-lang-item").with(lane_index).with(card_index);
                            drag_src(ui, id, |ui| {
                                card_ui::card_ui(ui, card);
                            });

                            if ui.memory().is_being_dragged(id) {
                                src_col_row = Some((LaneIndex::LaneId(lane_index), card_index));
                            }
                        }
                    })
                    .response;

                    dropped = dropped || ui.input().pointer.any_released();
                    if resp.hovered() {
                        dst_col_row = Some((LaneIndex::LaneId(lane_index), 0)); // TODO: dst row
                    }
                });
            });
        if lane.name.is_some() {
            // restore the lane name
            lane.name = Some(name);
        }
    }
    if dropped {
        if let Some((src_lane, src_card)) = src_col_row {
            match dst_col_row {
                Some((dst_lane, dst_card)) => {
                    on_drop.send(OnCardDrop {
                        src_lane,
                        dst_lane,
                        src_card,
                        dst_card,
                    });
                }
                None => {
                    on_remove.send(OnCardRemove { src_lane, src_card });
                }
            }
        }
    }
}

impl Plugin for CaoLangEditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<OnCardDrop>()
            .add_event::<OnCardRemove>()
            .insert_resource(CurrentProgram(CaoIr {
                // lanes: Vec::with_capacity(4),
                lanes: vec![
                    cao_lang::compiler::Lane::default(),
                    cao_lang::compiler::Lane::default()
                        .with_name("pog")
                        .with_card(Card::Pass)
                        .with_card(Card::Add)
                        .with_card(Card::Pass),
                ],
            }))
            .add_system_set(
                SystemSet::on_update(crate::AppState::CaoLangEditor)
                    .with_system(on_card_drop_system.system())
                    .with_system(on_card_remove_system.system())
                    .with_system(editor_ui_system.system()),
            );
    }
}
