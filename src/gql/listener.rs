use crate::gql::subscriptions::CreateReviewSubscription;
use crate::gql::Review;
use crate::settings::Settings;
use cynic::{GraphQlResponse, SubscriptionBuilder};
use futures::StreamExt;
use graphql_ws_client::Client;
use log::{debug, error, info, warn};
use std::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

pub struct ReviewListener {
    settings: Settings,
    tx: tokio::sync::mpsc::Sender<Review>,
}

impl ReviewListener {
    pub fn new(settings: Settings, tx: tokio::sync::mpsc::Sender<Review>) -> Self {
        Self { settings, tx }
    }

    pub async fn continuous_listen(&self) -> ! {
        loop {
            match self.listen().await {
                Ok(_) => {}
                Err(err) => {
                    error!("Error while listening for review creation: {}", err);
                    warn!("Trying again in 60 seconds");
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            }
        }
    }

    async fn listen(&self) -> anyhow::Result<()> {
        let mut req = self.settings.graphql.ws_url.clone().into_client_request()?;
        req.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_str("graphql-transport-ws")
                .expect("Could not transform header string to header value"),
        );

        info!(
            "Establishing websocket connection to {}",
            self.settings.graphql.ws_url
        );
        let (ws_stream, resp) = tokio_tungstenite::connect_async(req).await?;
        debug!("Websocket connection established: {:?}", resp);

        let gql = Client::build(ws_stream)
            .keep_alive_interval(Duration::from_secs(30))
            .keep_alive_retries(10)
            .subscription_buffer_size(32);

        let mut subscription = gql.subscribe(CreateReviewSubscription::build(())).await?;

        info!("Successfully subscribed to review creation");

        while let Some(msg) = subscription.next().await {
            match msg {
                Ok(msg) => {
                    self.handle_subscription_message(msg).await?;
                }
                Err(err) => {
                    error!("Error while receiving message from subscription: {}", err);
                    return Err(err.into());
                }
            }
        }

        Ok(())
    }

    async fn handle_subscription_message(
        &self,
        msg: GraphQlResponse<CreateReviewSubscription>,
    ) -> anyhow::Result<()> {
        debug!("Received message from subscription: {:?}", msg);

        let data = match msg.data {
            Some(data) => data,
            None => {
                if let Some(err) = msg.errors {
                    warn!(
                        "Error while receiving message from subscription: {:#?}",
                        err
                    );
                }
                return Ok(());
            }
        };

        if let Some(review) = data.review_created {
            self.tx.send(review).await?;
        } else {
            warn!(
                "Received message from subscription with unknown data: {:#?}",
                data
            );
        }

        Ok(())
    }
}
