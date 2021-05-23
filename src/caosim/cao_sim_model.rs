#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "ty", content = "payload")]
pub enum Message {
    Entities(EntitiesPayload),
    Terrain(TerrainPayload),
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct TerrainPayload {}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct EntitiesPayload {
    pub time: i64,
    pub room_id: AxialPos,
    pub bots: Vec<Bot>,
    pub structures: Vec<Structure>,
    pub resources: Vec<Resource>,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AxialPos {
    pub q: i32,
    pub r: i32,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Bot {
    pub id: i64,
    pub pos: AxialPos,
    pub carry: Option<Carry>,
    pub hp: Option<Hp>,
    pub script: Option<Script>,
    pub owner: Option<Owner>,
    pub decay: Option<Decay>,
    pub logs: Option<String>,
    pub say: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Carry {
    pub value: Option<i64>,
    pub value_max: i64,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Hp {
    pub value: i64,
    pub value_max: i64,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Script {
    pub data: String,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Owner {
    pub data: String,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Decay {
    pub hp_amount: i64,
    pub interval: i64,
    pub time_remaining: i64,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Structure {
    pub id: i64,
    pub pos: AxialPos,
    pub hp: Hp2,
    pub energy: Energy,
    pub energy_regen: i64,
    pub owner: Owner2,
    #[serde(rename = "StructureType")]
    pub structure_type: StructureType,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hp2 {
    pub value: i64,
    pub value_max: i64,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Energy {
    pub value: i64,
    pub value_max: i64,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Owner2 {
    pub data: String,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct StructureType {
    #[serde(rename = "Spawn")]
    pub spawn: Spawn,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Spawn {
    pub time_to_spawn: i64,
    pub spawning: i64,
    pub spawn_queue: Vec<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Resource {
    pub id: i64,
    pub pos: AxialPos,
    #[serde(rename = "ResourceType")]
    pub resource_type: ResourceType,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct ResourceType {
    #[serde(rename = "Energy")]
    pub energy: Energy2,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Energy2 {
    pub value: i64,
    pub value_max: i64,
}
