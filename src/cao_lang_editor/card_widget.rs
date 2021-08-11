use bevy_egui::egui::{self, DragValue, Response, Ui};
use cao_lang::{
    compiler::{Card, LaneNode},
    VarName,
};

use super::LaneNames;

fn lane_node_ui(ui: &mut Ui, node: &mut LaneNode, names: &LaneNames) {
    match node {
        LaneNode::LaneName(ref mut ln) => {
            egui::ComboBox::from_label("Lane")
                .selected_text(ln.as_str())
                .show_ui(ui, |ui| {
                    for name in names.0.iter() {
                        ui.selectable_value(ln, name.clone(), name);
                    }
                });
        }
        LaneNode::LaneId(i) => {
            egui::ComboBox::from_label("Lane")
                .selected_text(names.0.get(*i).map(|x| x.as_str()).unwrap_or(""))
                .show_ui(ui, |ui| {
                    for (j, name) in names.0.iter().enumerate() {
                        ui.selectable_value(i, j, name);
                    }
                });
        }
    }
}

pub fn card_ui(ui: &mut Ui, card: &mut Card, names: &LaneNames, error: Option<String>) -> Response {
    ui.scope(|ui| {
        let heading = egui::Label::new(card.name());
        let heading = if error.is_some() {
            heading.background_color(egui::Color32::RED).strong()
        } else {
            heading
        };
        let heading = ui.heading(heading);
        if let Some(error) = error {
            heading.on_hover_text(error);
        }
        match card {
            Card::SetGlobalVar(var)
            | Card::ReadVar(var)
            | Card::SetVar(var)
            | Card::SetProperty(var)
            | Card::GetProperty(var) => {
                ui.horizontal(|ui| {
                    ui.label("Variable ");
                    let mut payload = var.0.to_string();
                    if ui.text_edit_singleline(&mut payload).changed() {
                        if let Ok(res) = VarName::from(&payload) {
                            var.0 = res;
                        }
                    }
                });
            }
            Card::CallNative(node) => {
                ui.label(node.0.as_str());
            }
            Card::ScalarInt(node) => {
                ui.horizontal(|ui| {
                    ui.label("value:");
                    ui.add(DragValue::new(&mut node.0))
                });
            }
            Card::ScalarFloat(node) => {
                ui.horizontal(|ui| {
                    ui.label("value:");
                    ui.add(DragValue::new(&mut node.0))
                });
            }
            Card::StringLiteral(node) => {
                ui.text_edit_multiline(&mut node.0);
            }
            Card::IfElse { then, r#else } => {
                ui.label("then");
                lane_node_ui(ui, then, names);
                ui.label("else");
                lane_node_ui(ui, r#else, names);
            }
            Card::IfTrue(node)
            | Card::IfFalse(node)
            | Card::Jump(node)
            | Card::Repeat(node)
            | Card::While(node) => lane_node_ui(ui, node, names),
            // empty bodied items
            Card::Pass
            | Card::Add
            | Card::Sub
            | Card::Mul
            | Card::Div
            | Card::CopyLast
            | Card::Less
            | Card::LessOrEq
            | Card::Equals
            | Card::NotEquals
            | Card::Pop
            | Card::ClearStack
            | Card::And
            | Card::Or
            | Card::Xor
            | Card::Not
            | Card::Return
            | Card::ScalarNil
            | Card::CreateTable
            | Card::Abort => {}
        }
    })
    .response
}
