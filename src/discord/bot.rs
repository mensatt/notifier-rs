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

impl TypeMapKey for Settings {
    type Value = Arc<Settings>;
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

                let review_id = split[1];

                // We are gonna take a while, let's tell discord to calm down a bit
                match cmp.defer(ctx.http.clone()).await {
                    Ok(_) => {}
                    Err(e) => {
                        warn!("Failed to defer message: {}", e);
                        warn!("Message: {:#?}", cmp.message);
                        return;
                    }
                }

                match split[0] {
                    "approve" | "reject" => {
                        let approve = split[0] == "approve";

                        // Scope to minimize the time the lock is held
                        // (It shouldn't be an issue anyway, as it is only read, but better safe than sorry)
                        {
                            let gql_client = ctx.data.read().await;
                            let gql_client = gql_client
                                .get::<MensattGqlClient>()
                                .expect("Could not retrieve MensattGqlClient from global context");

                            match gql_client
                                .update_review(Uuid(review_id.to_string()), approve)
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

                        let msg_edit = EditMessage::new().components(get_action_row(
                            Some(approve),
                            false,
                            review_id,
                        ));

                        match cmp.message.edit(ctx.http.clone(), msg_edit).await {
                            Ok(_) => {}
                            Err(e) => {
                                warn!("Failed to edit message: {}", e);
                                warn!("Message: {:#?}", cmp.message);
                                return;
                            }
                        };
                    }
                    "rotate" => {
                        let angle = split[2].parse::<i32>().unwrap();

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

                        if let Some(image_id) = image_id {
                            {
                                let image_client = ctx.data.read().await;
                                let image_client = image_client
                                    .get::<ImageClient>()
                                    .expect("Could not retrieve ImageClient from global context");
                                match image_client.rotate_image(image_id, angle).await {
                                    Ok(_) => {
                                        info!(
                                            "Successfully rotated image {} by {}",
                                            image_id, angle
                                        );
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

                            // Update to fake image
                            let embed = cmp
                                .message
                                .embeds
                                .first()
                                .unwrap_or_else(|| {
                                    panic!("Received message without embed: {:#?}", cmp.message)
                                })
                                .clone();

                            let updated_embed = {
                                let settings = ctx.data.read().await;
                                let settings = settings
                                    .get::<Settings>()
                                    .expect("Could not retrieve Settings from global context");

                                CreateEmbed::from(embed).image(format!(
                                    "{}{}?auth={}&discord_fake={}",
                                    settings.image.image_url,
                                    image_id,
                                    settings.image.key,
                                    rand::random::<u64>()
                                ))
                            };

                            match cmp
                                .message
                                .edit(
                                    ctx.http.clone(),
                                    EditMessage::new().embeds(vec![updated_embed]),
                                )
                                .await
                            {
                                Ok(_) => {
                                    info!(
                                        "Successfully edited message on rotate: {:#?}",
                                        cmp.message
                                    );
                                }
                                Err(err) => {
                                    warn!("Failed to edit message on rotate: {}", err);
                                    warn!("Message: {:#?}", cmp.message);
                                    return;
                                }
                            };
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
                    _ => {
                        warn!(
                            "Received component interaction with invalid custom id: {}",
                            cmp.data.custom_id
                        );
                        warn!("Message: {:#?}", cmp.message);
                        return;
                    }
                }
            }
            _ => warn!("Received unknown interaction: {:#?}", interaction),
        }
    }
}

fn get_action_row(
    approved: Option<bool>,
    has_image: bool,
    review_id: &str,
) -> Vec<CreateActionRow> {
    let mut buttons: Vec<CreateButton> = vec![];

    let mut approve_btn = CreateButton::new("<invalid>")
        .emoji(ReactionType::Unicode("✅".to_string()))
        .style(ButtonStyle::Success);

    match approved {
        None => {
            approve_btn = approve_btn
                .label("Approve")
                .custom_id(format!("approve_{}", review_id));
        }
        Some(approved) => {
            if approved {
                approve_btn = approve_btn
                    .label("Approved")
                    .custom_id(format!("_____approve_{}", review_id))
                    .disabled(true);
            } else {
                approve_btn = approve_btn
                    .label("Approve")
                    .custom_id(format!("_____approve_{}", review_id))
                    .disabled(true);
            }
        }
    }

    let mut reject_btn = CreateButton::new("<invalid>")
        .emoji(ReactionType::Unicode("🗑".to_string()))
        .style(ButtonStyle::Danger);

    match approved {
        None => {
            reject_btn = reject_btn
                .label("Reject")
                .custom_id(format!("reject_{}", review_id));
        }
        Some(approved) => {
            if approved {
                reject_btn = reject_btn
                    .label("Reject")
                    .custom_id(format!("_____reject_{}", review_id))
                    .disabled(true);
            } else {
                reject_btn = reject_btn
                    .label("Rejected")
                    .custom_id(format!("_____reject_{}", review_id))
                    .disabled(true);
            }
        }
    }

    let mut rotation_btns = vec![
        CreateButton::new(format!("rotate_{}_270", review_id))
            .emoji(ReactionType::Unicode("↪".to_string()))
            .label(" ")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("rotate_{}_180", review_id))
            .emoji(ReactionType::Unicode("↕".to_string()))
            .label(" ")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("rotate_{}_90", review_id))
            .emoji(ReactionType::Unicode("↩".to_string()))
            .label(" ")
            .style(ButtonStyle::Secondary),
    ];

    buttons.push(approve_btn);

    // Only add rotations if it was not approved yet
    if has_image && approved != Some(true) {
        buttons.append(&mut rotation_btns);
    }

    buttons.push(reject_btn);

    vec![CreateActionRow::Buttons(buttons)]
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

            let msg = CreateMessage::new().embed(embed).components(get_action_row(
                None,
                !review.images.is_empty(),
                &review.id.to_string(),
            ));

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
            data.insert::<Settings>(Arc::new(self.settings.clone()));
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
