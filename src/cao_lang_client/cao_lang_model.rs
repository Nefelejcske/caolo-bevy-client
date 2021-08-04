use cao_lang::{
    compiler::{CallNode, Card},
    InputString,
};

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaNode {
    pub name: String,
    pub description: String,
    pub inputs: Vec<String>,
    pub ty: String,
    pub outputs: Vec<String>,
    pub properties: Vec<String>,
}

pub fn schema_to_card(node: &SchemaNode) -> Card {
    match node.ty.as_str() {
        "Undefined" => {
            panic!("Undefined card type");
        }
        "Branch" | "Object" | "Instruction" => match node.name.as_str() {
            "Pass" => Card::Pass,
            "Add" => Card::Add,
            "Sub" => Card::Sub,
            "Mul" => Card::Mul,
            "Div" => Card::Div,
            "CopyLast" => Card::CopyLast,
            "Less" => Card::Less,
            "LessOrEq" => Card::LessOrEq,
            "Equals" => Card::Equals,
            "NotEquals" => Card::NotEquals,
            "Pop" => Card::Pop,
            "ClearStack" => Card::ClearStack,
            "And" => Card::And,
            "Or" => Card::Or,
            "Xor" => Card::Xor,
            "Not" => Card::Not,
            "Abort" => Card::Abort,
            "ScalarNil" => Card::ScalarNil,
            "Return" => Card::Return,
            "CreateTable" => Card::CreateTable,
            "ScalarInt" => Card::ScalarInt(Default::default()),
            "ScalarFloat" => Card::ScalarFloat(Default::default()),
            "StringLiteral" => Card::StringLiteral(Default::default()),
            "IfTrue" => Card::IfTrue(Default::default()),
            "IfFalse" => Card::IfFalse(Default::default()),
            "Jump" => Card::Jump(Default::default()),
            "SetGlobalVar" => Card::SetGlobalVar(Default::default()),
            "ReadVar" => Card::ReadVar(Default::default()),
            "Repeat" => Card::Repeat(Default::default()),
            "While" => Card::While(Default::default()),
            "IfElse" => Card::IfElse {
                then: Default::default(),
                r#else: Default::default(),
            },
            "SetProperty" => Card::SetProperty(Default::default()),
            "GetProperty" => Card::GetProperty(Default::default()),
            _ => todo!("Schema name {} is not implemented", node.name),
        },
        "Call" => Card::CallNative(Box::new(CallNode(
            InputString::from(node.name.as_str()).expect("function name is too long"),
        ))),
        _ => todo!("Unknown card type {}", node.ty),
    }
}