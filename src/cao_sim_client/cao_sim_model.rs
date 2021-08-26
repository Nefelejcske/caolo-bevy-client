use super::hex_axial_to_pixel;

#[derive(serde::Serialize)]
pub struct GetLayoutQuery {
    pub radius: i32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "ty", content = "payload")]
pub enum Message {
    Entities(EntitiesPayload),
    Terrain(Option<TerrainPayload>),
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainPayload {
    pub room_id: AxialPos,
    pub offset: AxialPos,
    pub tiles: Vec<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainTy {
    Empty,
    Plain,
    Wall,
    Bridge,
}

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

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Eq, Hash)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct EntityPosition {
    pub room: AxialPos,
    pub pos: AxialPos,
    pub offset: AxialPos,
}

#[derive(
    Copy, Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Eq, Hash,
)]
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
    pub pos: EntityPosition,
    pub carry: Option<Carry>,
    pub hp: Option<Hp>,
    pub script: Option<Script>,
    pub owner: Option<Owner>,
    pub decay: Option<Decay>,
    pub logs: Option<String>,
    pub say: Option<String>,
    pub mine_intent: Option<MineIntent>,
    pub dropoff_intent: Option<DropoffIntent>,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DropoffIntent {
    pub target_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MineIntent {
    pub target_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Carry {
    pub value: i64,
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
    pub pos: EntityPosition,
    pub hp: Hp,
    pub energy: Energy,
    pub energy_regen: i64,
    pub owner: Owner,
    #[serde(rename = "StructureType")]
    pub structure_type: StructureType,
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
    pub pos: EntityPosition,
    #[serde(rename = "ResourceType")]
    pub resource_type: ResourceType,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct ResourceType {
    #[serde(rename = "Energy")]
    pub energy: Energy,
}

impl EntityPosition {
    pub fn as_pixel(&self) -> bevy::math::Vec2 {
        let q = self.pos.q + self.offset.q;
        let r = self.pos.r + self.offset.r;
        hex_axial_to_pixel(q as f32, r as f32)
    }

    pub fn absolute_axial(&self) -> AxialPos {
        AxialPos {
            q: self.pos.q + self.offset.q,
            r: self.pos.r + self.offset.r,
        }
    }
}
