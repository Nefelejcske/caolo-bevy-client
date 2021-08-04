use bevy_egui::egui::{DragValue, Ui};
use cao_lang::{
    compiler::{Card, LaneNode},
    VarName,
};

fn lane_node_ui(node: &mut LaneNode) {
    // TODO
}

pub fn card_ui(ui: &mut Ui, card: &mut Card) {
    ui.heading(card.name());
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
            lane_node_ui(then);
            ui.label("else");
            lane_node_ui(r#else);
        }
        Card::IfTrue(node)
        | Card::IfFalse(node)
        | Card::Jump(node)
        | Card::Repeat(node)
        | Card::While(node) => lane_node_ui(node),
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
}
