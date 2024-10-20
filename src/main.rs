#![allow(dead_code)]

use crate::gql::subscriptions::Review;
use crate::settings::Settings;
use config::Config;
use log::{debug, error, info};

mod discord;
mod gql;
mod settings;

#[tokio::main]
async fn main() {
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

    let (tx, rx) = tokio::sync::mpsc::channel::<Review>(8);

    // Create GQL listener
    let gql_task = tokio::spawn(async move {
        let listener = gql::listener::ReviewListener::new(settings.graphql.ws_url, tx);

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
        let mut bot = discord::bot::Bot::new(rx, settings.discord.comm_channel);
        bot.start(settings.discord.token.as_str())
            .await
            .expect("Failed to start bot");
    });

    info!("Notifier service started!");

    discord_handle.await.expect("Discord bot failed");
    gql_task.await.expect("GQL listener failed");
}
