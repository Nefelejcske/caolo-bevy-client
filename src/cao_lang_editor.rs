mod card_widget;
mod lane_widget;

use crate::cao_lang_client::{
    cao_lang_model::{schema_to_card, RemoteCompileError},
    CaoLangSchema,
};
use bevy::{prelude::*, tasks::Task};
use bevy_egui::{
    egui::{self, color, CursorIcon, Id, InnerResponse, LayerId, Order, Sense, Shape, Ui},
    EguiContext,
};
use cao_lang::compiler::{CaoIr, Card, CompilationError};
use futures_lite::future;

pub struct CurrentProgram(pub CaoIr);
pub struct LaneNames(pub Vec<String>);
pub struct CurrentLocalCompileError(pub Option<CompilationError>);
pub struct CurrentRemoteCompileError(pub Option<RemoteCompileError>);

type LocalCompileResult = Result<CaoIr, CompilationError>;
type RemoteCompileResult = Result<(), RemoteCompileError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

pub struct CaoLangEditorPlugin;

fn drag_src<R>(ui: &mut Ui, id: Id, mut body: impl FnMut(&mut Ui) -> R) {
    let is_being_dragged = ui.memory().is_being_dragged(id);

    if !is_being_dragged || ui.ctx().input().pointer.any_click() {
        // `any_click` is used to allow interaction with the body
        // once https://github.com/emilk/egui/issues/547 is fixed we shouldn't need it
        //
        let response = ui.scope(&mut body).response;
        let _response = ui.interact(response.rect, id, Sense::drag());
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
    stroke: impl Into<Option<egui::Stroke>>,
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
    let mut stroke = stroke.into().unwrap_or(style.bg_stroke);
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

fn on_card_drop_system(
    mut ir: ResMut<CurrentProgram>,
    schema: Res<CaoLangSchema>,
    mut on_drop: EventReader<OnCardDrop>,
) {
    let lanes = &mut ir.0.lanes;
    for drop in on_drop.iter().copied() {
        debug!("Drop event {:?}", drop);

        let OnCardDrop {
            src_lane,
            dst_lane,
            src_card,
            dst_card,
        } = drop;

        let card: Card = match src_lane {
            LaneIndex::LaneId(id) if lanes[id].cards.len() > src_card => {
                lanes[id].cards.remove(src_card)
            }
            LaneIndex::SchemaLane => schema_to_card(&schema.0[src_card]),
            _ => {
                continue;
            }
        };

        match dst_lane {
            LaneIndex::LaneId(id) => {
                lanes[id].cards.insert(dst_card, card);
            }
            LaneIndex::SchemaLane => { /*noop*/ }
        }
    }
}

fn schema_ui(
    egui_ctx: &mut EguiContext,
    schema: &CaoLangSchema,
    src_col_row: &mut Option<(LaneIndex, usize)>,
    dst_col_row: &mut Option<(LaneIndex, usize)>,
    dropped: &mut bool,
) {
    egui::Window::new("Schema")
        .scroll(true)
        .id(egui::Id::new("cao-lang-schema"))
        .show(egui_ctx.ctx(), |ui| {
            ui.columns(1, |uis| {
                let ui = &mut uis[0];
                let resp = drop_target(
                    ui,
                    true,
                    |ui| {
                        for (card_index, card) in schema.0.iter().enumerate() {
                            let id = Id::new("cao-lang-schema-item").with(card_index);
                            drag_src(ui, id, |ui| {
                                card_widget::schema_card_ui(ui, card);
                            });

                            if ui.memory().is_being_dragged(id) {
                                *src_col_row = Some((LaneIndex::SchemaLane, card_index));
                            }
                        }
                    },
                    None,
                )
                .response;

                *dropped = *dropped || ui.input().pointer.any_released();
                if resp.hovered() {
                    *dst_col_row = Some((LaneIndex::SchemaLane, 0));
                }
            });
        });
}

fn left_ui_system(
    egui_ctx: ResMut<EguiContext>, // exclusive ownership
    compile_error: Res<CurrentLocalCompileError>,
    remote_compile_error: Res<CurrentRemoteCompileError>,
    mut ir: ResMut<CurrentProgram>,
) {
    egui::SidePanel::left("cao-lang-control").show(egui_ctx.ctx(), |ui| {
        ui.heading("Compilation result");
        match compile_error.0.as_ref() {
            Some(err) => {
                ui.colored_label(egui::color::Rgba::RED, err.payload.to_string());
            }
            None => {
                ui.colored_label(egui::color::Rgba::GREEN, "Success");
            }
        }
        ui.separator();
        ui.heading("Remote Compilation result");
        match remote_compile_error.0.as_ref() {
            Some(err) => {
                ui.colored_label(egui::color::Rgba::RED, err.detail.clone());
            }
            None => {
                ui.colored_label(egui::color::Rgba::GREEN, "Success");
            }
        }
        ui.separator();

        if ui.small_button("Add Lane").clicked() {
            ir.0.lanes.push(Default::default());
        }
    });
}

fn remote_compile_result_system(
    mut cmd: Commands,
    tasks: Query<(Entity, &mut Task<RemoteCompileResult>)>,
    mut compile_error: ResMut<CurrentRemoteCompileError>,
) {
    tasks.for_each_mut(|(e, mut task)| {
        if let Some(res) = future::block_on(future::poll_once(&mut *task)) {
            match res {
                Ok(_) => compile_error.0 = None,
                Err(err) => compile_error.0 = Some(err),
            }
            cmd.entity(e).despawn_recursive();
        }
    });
}

fn compiler_result_system(
    mut cmd: Commands,
    tasks: Query<(Entity, &mut Task<LocalCompileResult>)>,
    mut compile_error: ResMut<CurrentLocalCompileError>,
    pool: Res<bevy::tasks::IoTaskPool>,
) {
    tasks.for_each_mut(|(e, mut task)| {
        if let Some(res) = future::block_on(future::poll_once(&mut *task)) {
            match res {
                Ok(ir) => {
                    debug!("Sending IR to server");
                    cmd.spawn()
                        .insert(pool.spawn(crate::cao_lang_client::compile_program(ir)));

                    compile_error.0 = None;
                }
                Err(err) => compile_error.0 = Some(err),
            }
            cmd.entity(e).despawn_recursive();
        }
    });
}

// TODO: trigger on an event?
fn compiler_system(
    mut cmd: Commands,
    ir: Res<CurrentProgram>,
    tasks: Query<
        (),
        Or<(
            With<Task<LocalCompileResult>>,
            With<Task<RemoteCompileResult>>,
        )>,
    >,
    pool: Res<bevy::tasks::AsyncComputeTaskPool>,
) {
    if tasks.iter().next().is_some() {
        // compilation task is in progress
        return;
    }

    let ir = ir.0.clone();
    let task = pool.spawn(async move {
        let result = cao_lang::compiler::compile(&ir, None);
        result.map(|_| ir)
    });

    cmd.spawn().insert(task);
}

fn update_lane_names_system(ir: Res<CurrentProgram>, mut names: ResMut<LaneNames>) {
    names.0 =
        ir.0.lanes
            .iter()
            .map(|lane| lane.name.as_ref().map(|x| x.as_str()).unwrap_or(""))
            .map(|x| x.to_string())
            .collect();
}

fn editor_ui_system(
    mut egui_ctx: ResMut<EguiContext>, // exclusive ownership
    schema: Res<CaoLangSchema>,
    mut ir: ResMut<CurrentProgram>,
    lane_names: Res<LaneNames>,
    mut on_drop: EventWriter<OnCardDrop>,
    compile_error: Res<CurrentLocalCompileError>,
) {
    let mut src_col_row = None;
    let mut dst_col_row = None;
    let mut dropped = false;

    schema_ui(
        &mut *egui_ctx,
        &*schema,
        &mut src_col_row,
        &mut dst_col_row,
        &mut dropped,
    );
    let mut closed_lane_idx = None;
    for (lane_index, lane) in ir.0.lanes.iter_mut().enumerate() {
        let mut open = true;
        lane_widget::lane_ui(
            lane,
            lane_index,
            &*lane_names,
            &mut *egui_ctx,
            &mut src_col_row,
            &mut dst_col_row,
            &mut dropped,
            &*compile_error,
            &mut open,
        );
        if !open {
            closed_lane_idx = Some(lane_index);
        }
    }
    if dropped {
        if let Some(((src_lane, src_card), (dst_lane, dst_card))) =
            src_col_row.into_iter().zip(dst_col_row.into_iter()).next()
        {
            on_drop.send(OnCardDrop {
                src_lane,
                dst_lane,
                src_card,
                dst_card,
            });
        }
    }
    if let Some(i) = closed_lane_idx {
        ir.0.lanes.remove(i);
    }
}

impl Plugin for CaoLangEditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<OnCardDrop>()
            .insert_resource(LaneNames(Vec::with_capacity(4)))
            .insert_resource(CurrentLocalCompileError(None))
            .insert_resource(CurrentRemoteCompileError(None))
            .insert_resource(CurrentProgram(CaoIr {
                lanes: vec![cao_lang::compiler::Lane::default().with_name("Main")],
            }))
            .add_system_set(
                SystemSet::on_update(crate::AppState::CaoLangEditor)
                    .with_system(left_ui_system.system())
                    .with_system(on_card_drop_system.system())
                    .with_system(update_lane_names_system.system())
                    .with_system(compiler_system.system())
                    .with_system(compiler_result_system.system())
                    .with_system(remote_compile_result_system.system())
                    .with_system(editor_ui_system.system()),
            );
    }
}
