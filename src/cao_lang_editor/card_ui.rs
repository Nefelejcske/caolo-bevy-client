use bevy_egui::egui::Ui;
use cao_lang::compiler::Card;

pub fn card_ui(ui: &mut Ui, card: &mut Card) {
    ui.heading(card.name());
    match card {
        Card::SetProperty(_) => {}
        Card::GetProperty(_) => {}
        Card::ScalarInt(_) => {}
        Card::ScalarFloat(_) => {}
        Card::StringLiteral(_) => {}
        Card::CallNative(_) => {}
        Card::IfTrue(_) => {}
        Card::IfFalse(_) => {}
        Card::IfElse { then, r#else } => {}
        Card::Jump(_) => {}
        Card::SetGlobalVar(_) => {}
        Card::SetVar(_) => {}
        Card::ReadVar(_) => {}
        Card::Repeat(_) => {}
        Card::While(_) => {}
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
