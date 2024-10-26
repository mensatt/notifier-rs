#![allow(dead_code)]

use crate::gql::subscriptions::Review;
use crate::settings::Settings;
use config::Config;
use log::{debug, error, info};
use rustls::crypto::CryptoProvider;

mod discord;
mod gql;
mod image;
mod settings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    info!("Creating local graphql and image client");
    let mut gql_client = gql::client::MensattGqlClient::new(settings.clone());
    gql_client.login().await?;
    let image_client = image::ImageClient::new(settings.clone());

    // Buffer size shouldn't really matter here, as I don't expect the receiver to take that long
    let (tx, rx) = tokio::sync::mpsc::channel::<Review>(8);

    // Required as both futures use async move, thus they "invalidate" settings
    // Consequently, we give one future one clone and the other one the original
    // I think it would also be fine to just pass references, but that could involve lifetimes
    let settings_dup = settings.clone();

    // Create GQL listener
    let gql_task = tokio::spawn(async move {
        let listener = gql::listener::ReviewListener::new(settings_dup, tx);

        let mut tries: u32 = 0;

        loop {
            let res = listener.listen().await;
            match res {
                Ok(_) => {
                    error!("Listener returned?");
                    panic!();
                }
                Err(err) => {
                    error!("Listener error: {}", err);
                    error!("Retrying in 60 seconds...");
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            tries += 1;

            if tries > 10 {
                panic!("Too many tries, exiting...");
            }
        }
    });

    // Create discord bot
    let discord_handle = tokio::spawn(async move {
        let mut bot = discord::bot::Bot::new(rx, settings.clone());
        bot.start(settings.discord.token.as_str(), gql_client, image_client)
            .await
            .expect("Failed to start bot");
    });

    info!("Notifier service started!");

    // Tasks finishing is equivalent to them failing, as they should run forever
    discord_handle.await.expect("Discord bot failed");
    gql_task.await.expect("GQL listener failed");

    Ok(())
}
