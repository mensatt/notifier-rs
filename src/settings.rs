use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub discord: Discord,
    pub graphql: GraphQl,
    pub mensatt: Mensatt,
    pub image: Image,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Discord {
    pub token: String,
    pub comm_channel: u64,
    pub guilds: Vec<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GraphQl {
    pub ws_url: String,
    pub https_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Mensatt {
    pub occurrence_url: String,
    pub user: String,
    pub password: String,
    // Threshold how far before actual expiration a token should be treated as expired
    // This is used to avoid potential race conditions or slight clock desyncs
    pub jwt_threshold_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Image {
    pub image_url: String,
    pub rotate_url: String,
    pub key: String,
}
