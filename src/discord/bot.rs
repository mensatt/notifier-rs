use crate::gql::subscriptions::Review;
use log::{debug, info};
use serenity::all::{Context, EventHandler, GatewayIntents, Ready};
use serenity::{async_trait, Client};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, data_about_bot: Ready) {
        info!(
            "Received discord bot ready event, we are: {}",
            data_about_bot.user.name
        );
        debug!("{:#?}", data_about_bot);
    }
}

pub struct Bot {
    rx: tokio::sync::mpsc::Receiver<Review>,
}

impl Bot {
    pub fn new(rx: tokio::sync::mpsc::Receiver<Review>) -> Self {
        Bot { rx }
    }

    pub async fn start(&mut self, token: &str) -> anyhow::Result<()> {
        let intents = GatewayIntents::empty();

        info!("Starting Discord bot...");
        let mut client = Client::builder(token, intents)
            .event_handler(Handler)
            .await
            .expect("Failed to create client");

        client.start().await.expect("Failed to start client");

        info!("Discord bot started!");

        info!("Waiting for review events...");
        while let Some(review) = self.rx.recv().await {
            info!("Received review through channel: {:#?}", review);
        }

        Ok(())
    }
}
