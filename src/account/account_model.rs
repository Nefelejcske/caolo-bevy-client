#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoginError {
    pub detail: String,
}

/// Returns the token
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoginSuccess {
    pub access_token: super::AuthToken,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LoginUnprocEntity {
    pub detail: Vec<Detail>,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Detail {
    pub loc: Vec<String>,
    pub msg: String,
    #[serde(rename = "type")]
    pub type_field: String,
}
