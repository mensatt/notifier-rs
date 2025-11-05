#![allow(dead_code)]

use crate::gql::Review;
use crate::settings::Settings;
use config::Config;
use log::{debug, info};
use rustls::crypto::CryptoProvider;

mod discord;
mod gql;
mod image;
mod settings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TODO: This (and the direct dependency on rustls) can be removed if/once
    // all dependencies use aws-lc-rs
    CryptoProvider::install_default(rustls::crypto::aws_lc_rs::default_provider())
        .expect("Could not install default crypto provider");

    pretty_env_logger::formatted_timed_builder()
        .filter(Some("notifier_rs"), log::LevelFilter::Info)
        .try_init()
        .expect("Could not initialize logger");

    let config = Config::builder()
        .add_source(config::File::with_name("config.toml").required(true))
        .build()
        .expect("Could not load config");
    debug!("Loaded config: {:?}", config);

    let settings: Settings = config
        .try_deserialize()
        .expect("Could not deserialize settings");
    debug!("Loaded settings: {:#?}", settings);

    info!("Starting up notifier service...");

    // Buffer size shouldn't really matter here, as I don't expect the receiver to take that long
    let (tx, rx) = tokio::sync::mpsc::channel::<Review>(8);

    // Required as both futures use async move, thus they "invalidate" settings
    // Consequently, we give one future one clone and the other one the original
    // I think it would also be fine to just pass references, but that could involve lifetimes
    let settings_dup = settings.clone();

    // Create GQL listener
    let gql_task = tokio::spawn(async move {
        let listener = gql::listener::ReviewListener::new(settings_dup, tx);
        listener.continuous_listen().await;
    });

    // Create discord bot
    let discord_handle = tokio::spawn(async move {
        discord::bot::Bot::new(rx, settings.clone())
            .start()
            .await
            .expect("Failed to start bot");
    });

    info!("Notifier service started!");

    // Tasks finishing is equivalent to them failing, as they should run forever
    discord_handle.await.expect("Discord bot failed");
    gql_task.await.expect("GQL listener failed");

    Ok(())
}
