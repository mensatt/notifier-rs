use crate::gql::subscriptions::Review;
use log::{debug, info, warn};
use serenity::all::{
    ButtonStyle, Colour, Context, CreateButton, CreateEmbed, CreateEmbedAuthor, CreateMessage,
    EventHandler, GatewayIntents, Interaction, ReactionType, Ready, Timestamp,
};
use serenity::builder::CreateActionRow;
use serenity::model::id::ChannelId;
use serenity::{async_trait, Client};
use std::str::FromStr;

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

    async fn interaction_create(&self, _ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(cmd) => {
                info!("Received command interaction: {:#?}", cmd);
            }
            Interaction::Component(cmp) => {
                info!("Received component interaction: {:#?}", cmp);
            }
            _ => warn!("Received unknown interaction: {:#?}", interaction),
        }
    }
}

pub struct Bot {
    rx: tokio::sync::mpsc::Receiver<Review>,
    comm_channel: u64,
}

impl Bot {
    pub fn new(rx: tokio::sync::mpsc::Receiver<Review>, comm_channel: u64) -> Self {
        Bot { rx, comm_channel }
    }

    pub async fn start(&mut self, token: &str) -> anyhow::Result<()> {
        let intents = GatewayIntents::empty();

        info!("Starting Discord bot...");
        let mut client = Client::builder(token, intents)
            .event_handler(Handler)
            .await
            .expect("Failed to create client");

        let http = client.http.clone();
        tokio::spawn(async move {
            client.start().await.expect("Failed to start client");
        });

        info!("Discord bot started!");
        info!("Waiting for review events...");

        let comms = ChannelId::new(self.comm_channel);

        while let Some(review) = self.rx.recv().await {
            info!("Received review through channel: {:#?}", review);

            let mut embed = CreateEmbed::new()
                .author(CreateEmbedAuthor::new(
                    review.display_name.unwrap_or("Anonymous".to_string()),
                ))
                .colour(Colour::from_rgb(255, 107, 38))
                .timestamp(
                    Timestamp::from_str(review.created_at.0.as_str()).unwrap_or_else(|_| {
                        panic!("Could not parse review time stamp: {:?}", review.created_at)
                    }),
                )
                .title(format!(
                    "{} | {}",
                    review.occurrence.dish.name_de,
                    (0..review.stars).map(|_| '★').collect::<String>()
                ))
                .url(format!(
                    "https://mensatt.de/details/{}",
                    review.occurrence.id.0
                ));

            if let Some(text) = review.text {
                embed = embed.description(text);
            }

            if !review.images.is_empty() {
                embed = embed.image(format!(
                    "https://api.mensatt.de/content/image/{}",
                    review.images.first().unwrap().id.0
                ));
            }

            let msg = CreateMessage::new()
                .embed(embed)
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new(format!("approve-{}", review.id))
                        .emoji(ReactionType::Unicode("✅".to_string()))
                        .label("Approve")
                        .style(ButtonStyle::Success),
                    CreateButton::new(format!("reject-{}", review.id))
                        .emoji(ReactionType::Unicode("🗑".to_string()))
                        .label("Reject")
                        .style(ButtonStyle::Danger),
                ])]);

            comms.send_message(http.clone(), msg).await?;
        }

        Ok(())
    }
}
