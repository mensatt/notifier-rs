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
}

#[derive(Debug, Deserialize, Clone)]
pub struct Image {
    pub image_url: String,
    pub key: String,
}
