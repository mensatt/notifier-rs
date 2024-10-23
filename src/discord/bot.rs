use crate::gql::client::MensattGqlClient;
use crate::gql::subscriptions::Review;
use crate::gql::Uuid;
use crate::settings::Settings;
use log::{debug, info, warn};
use serenity::all::{
    ButtonStyle, Colour, Context, CreateButton, CreateEmbed, CreateEmbedAuthor, CreateMessage,
    EditMessage, EventHandler, GatewayIntents, Http, Interaction, ReactionType, Ready, Timestamp,
};
use serenity::builder::CreateActionRow;
use serenity::model::id::ChannelId;
use serenity::prelude::TypeMapKey;
use serenity::{async_trait, Client};
use std::str::FromStr;
use std::sync::Arc;

struct Handler;

impl TypeMapKey for MensattGqlClient {
    type Value = Arc<MensattGqlClient>;
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, data_about_bot: Ready) {
        info!(
            "Received discord bot ready event, we are: {}",
            data_about_bot.user.name
        );
        debug!("{:#?}", data_about_bot);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(cmd) => {
                info!("Received command interaction: {:#?}", cmd);
            }
            Interaction::Component(mut cmp) => {
                info!("Received component interaction: {:#?}", cmp);

                let split = cmp.data.custom_id.split("_").collect::<Vec<_>>();
                if split.len() != 2 {
                    warn!(
                        "Received component interaction with invalid custom id: {}",
                        cmp.data.custom_id
                    );
                    return;
                }

                let approve = split[0] == "approve";
                let id = split[1];

                match cmp.defer(ctx.http.clone()).await {
                    Ok(_) => {}
                    Err(e) => {
                        warn!("Failed to defer message: {}", e);
                        warn!("Message: {:#?}", cmp.message);
                        return;
                    }
                }

                // Scope to minimize the time the lock is held
                // (It shouldn't be an issue anyway, as it is only read, but better safe than sorry)
                {
                    let gql_client = ctx.data.read().await;
                    let gql_client = gql_client
                        .get::<MensattGqlClient>()
                        .expect("Could not retrieve MensattGqlClient from global context");

                    match gql_client
                        .update_review(Uuid(id.to_string()), approve)
                        .await
                    {
                        Ok(_) => {}
                        Err(err) => {
                            warn!("Failed to update review: {}", err);
                            warn!("Original message: {:#?}", cmp.message);
                            return;
                        }
                    };
                }

                // TODO: Deduplicate button creation
                let msg_edit = EditMessage::new().components(vec![CreateActionRow::Buttons(vec![
                    // Hack: Add '_' before the custom id, to make it fail if for some reason it
                    // is clicked after being disabled
                    CreateButton::new(format!("_approve_{}", id))
                        .emoji(ReactionType::Unicode("✅".to_string()))
                        .label(if approve {
                            format!("Approved by {}", cmp.user.name)
                        } else {
                            "Approve".to_string()
                        })
                        .disabled(true)
                        .style(ButtonStyle::Success),
                    CreateButton::new(format!("_reject_{}", id))
                        .emoji(ReactionType::Unicode("🗑".to_string()))
                        .label(if !approve { "Rejected" } else { "Reject" })
                        .disabled(true)
                        .style(ButtonStyle::Danger),
                ])]);

                match cmp.message.edit(ctx.http.clone(), msg_edit).await {
                    Ok(_) => {}
                    Err(e) => {
                        warn!("Failed to edit message: {}", e);
                        warn!("Message: {:#?}", cmp.message);
                        return;
                    }
                };
            }
            _ => warn!("Received unknown interaction: {:#?}", interaction),
        }
    }
}

pub struct Bot {
    rx: tokio::sync::mpsc::Receiver<Review>,
    settings: Settings,
}

impl Bot {
    pub fn new(rx: tokio::sync::mpsc::Receiver<Review>, settings: Settings) -> Self {
        Bot { rx, settings }
    }

    pub async fn listen_for_gql_events(&mut self, http: Arc<Http>) -> anyhow::Result<()> {
        let comms = ChannelId::new(self.settings.discord.comm_channel);

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
                    "{}{}",
                    self.settings.mensatt.occurrence_url, review.occurrence.id.0
                ));

            if let Some(text) = review.text {
                embed = embed.description(text);
            }

            if !review.images.is_empty() {
                embed = embed.image(format!(
                    "{}{}",
                    self.settings.image.image_url,
                    review.images.first().unwrap().id.0
                ));
            }

            let msg = CreateMessage::new()
                .embed(embed)
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new(format!("approve_{}", review.id))
                        .emoji(ReactionType::Unicode("✅".to_string()))
                        .label("Approve")
                        .style(ButtonStyle::Success),
                    CreateButton::new(format!("reject_{}", review.id))
                        .emoji(ReactionType::Unicode("🗑".to_string()))
                        .label("Reject")
                        .style(ButtonStyle::Danger),
                ])]);

            comms.send_message(http.clone(), msg).await?;
        }

        Ok(())
    }

    // Note: I would have liked to make the gql_client a member of the Bot struct, but this
    // doesn't seem to work in combination with serenity-rs's implementation of bot global state.
    // Moving it into the TypeMap would invalide the self.gql_client member and is not allowed.
    pub async fn start(
        &mut self,
        token: &str,
        mensatt_gql_client: MensattGqlClient,
    ) -> anyhow::Result<()> {
        let intents = GatewayIntents::empty();

        info!("Starting Discord bot...");
        let mut client = Client::builder(token, intents)
            .event_handler(Handler)
            .await
            .expect("Failed to create client");

        {
            let mut data = client.data.write().await;
            data.insert::<MensattGqlClient>(Arc::new(mensatt_gql_client));
        }

        let http = client.http.clone();
        tokio::spawn(async move {
            client.start().await.expect("Failed to start client");
        });

        info!("Discord bot started!");
        info!("Waiting for review events...");

        self.listen_for_gql_events(http).await?;

        Ok(())
    }
}
