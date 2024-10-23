use crate::gql::client::MensattGqlClient;
use crate::gql::subscriptions::Review;
use crate::gql::Uuid;
use crate::image::ImageClient;
use crate::settings::Settings;
use log::{debug, info, warn};
use serenity::all::{
    ButtonStyle, Colour, Context, CreateButton, CreateEmbed, CreateEmbedAuthor,
    CreateInteractionResponseFollowup, CreateMessage, EditMessage, EventHandler, GatewayIntents,
    Http, Interaction, ReactionType, Ready, Timestamp,
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

impl TypeMapKey for ImageClient {
    type Value = Arc<ImageClient>;
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
                if split.len() < 2 || split.len() > 3 {
                    warn!(
                        "Received component interaction with invalid custom id: {}",
                        cmp.data.custom_id
                    );
                    return;
                }

                let (approve, angle) = match split[0] {
                    "approve" => (true, None),
                    "reject" => (false, None),
                    "rotate" => (true, Some(split[2].parse::<i32>().unwrap())),
                    _ => {
                        warn!(
                            "Received component interaction with invalid custom id: {}",
                            cmp.data.custom_id
                        );
                        warn!("Message: {:#?}", cmp.message);
                        return;
                    }
                };

                let id = split[1];

                match cmp.defer(ctx.http.clone()).await {
                    Ok(_) => {}
                    Err(e) => {
                        warn!("Failed to defer message: {}", e);
                        warn!("Message: {:#?}", cmp.message);
                        return;
                    }
                }

                let image_id: Option<&str> = {
                    if let Some(embed) = cmp.message.embeds.first() {
                        if let Some(image) = embed.image.as_ref() {
                            Some(
                                image
                                    .url
                                    .split("/")
                                    .last()
                                    .unwrap_or_else(|| {
                                        panic!("Could not split image url: {}", image.url)
                                    })
                                    .split("?")
                                    .next()
                                    .unwrap_or_else(|| {
                                        panic!("Could not split image url: {}", image.url)
                                    }),
                            )
                        } else {
                            None // I don't know why None at the end isn't enough
                        }
                    } else {
                        None
                    }
                };

                if let Some(angle) = angle {
                    if let Some(image_id) = image_id {
                        {
                            let image_client = ctx.data.read().await;
                            let image_client = image_client
                                .get::<ImageClient>()
                                .expect("Could not retrieve ImageClient from global context");
                            match image_client.rotate_image(image_id, angle).await {
                                Ok(_) => {
                                    info!("Successfully rotated image {} by {}", image_id, angle);
                                }
                                Err(err) => {
                                    warn!(
                                        "Failed to rotate image {} by {}: {}",
                                        image_id, angle, err
                                    );
                                    warn!("Message: {:#?}", cmp.message);
                                    return;
                                }
                            };
                        }
                    } else {
                        info!("Tried to rotate image without image id!");
                        debug!("Message: {:#?}", cmp.message);
                        match cmp
                            .create_followup(
                                ctx.http.clone(),
                                CreateInteractionResponseFollowup::new()
                                    .ephemeral(true)
                                    .content("You cannot rotate nothing! 😠"),
                            )
                            .await
                        {
                            Ok(_) => {}
                            Err(err) => {
                                warn!("Failed to create followup: {}", err);
                                warn!("Original message: {:#?}", cmp.message);
                            }
                        };
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
                    CreateButton::new(format!("_____approve_{}", id))
                        .emoji(ReactionType::Unicode("✅".to_string()))
                        .label(if approve {
                            format!(
                                "Approved by {}{}",
                                cmp.user.name,
                                if let Some(angle) = angle {
                                    format!(" ({}°)", angle)
                                } else {
                                    "".to_string()
                                }
                            )
                        } else {
                            "Approve".to_string()
                        })
                        .disabled(true)
                        .style(ButtonStyle::Success),
                    CreateButton::new(format!("_____reject_{}", id))
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
                    "{}{}?auth={}",
                    self.settings.image.image_url,
                    review.images.first().unwrap().id.0,
                    self.settings.image.key
                ));
            }

            let msg = CreateMessage::new()
                .embed(embed)
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new(format!("approve_{}", review.id))
                        .emoji(ReactionType::Unicode("✅".to_string()))
                        .label("Approve")
                        .style(ButtonStyle::Success),
                    CreateButton::new(format!("rotate_{}_270", review.id))
                        .emoji(ReactionType::Unicode("⬅".to_string()))
                        .label("Rotate left")
                        .style(ButtonStyle::Secondary),
                    CreateButton::new(format!("rotate_{}_180", review.id))
                        .emoji(ReactionType::Unicode("↕".to_string()))
                        .label("Flip")
                        .style(ButtonStyle::Secondary),
                    CreateButton::new(format!("rotate_{}_90", review.id))
                        .emoji(ReactionType::Unicode("➡".to_string()))
                        .label("Rotate right")
                        .style(ButtonStyle::Secondary),
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
        image_client: ImageClient,
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
            data.insert::<ImageClient>(Arc::new(image_client));
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
