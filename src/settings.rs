use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub discord: Discord,
    pub graphql: GraphQl,
}

#[derive(Debug, Deserialize)]
pub struct Discord {
    pub token: String,
    pub comm_channel: u64,
}

#[derive(Debug, Deserialize)]
pub struct GraphQl {
    pub ws_url: String,
}
