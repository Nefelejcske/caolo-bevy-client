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
