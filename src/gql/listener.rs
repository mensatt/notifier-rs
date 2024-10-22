use crate::gql::subscriptions::{CreateReviewSubscription, Review};
use cynic::{GraphQlResponse, SubscriptionBuilder};
use futures::StreamExt;
use graphql_ws_client::Client;
use log::{debug, error, info, warn};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

pub struct ReviewListener {
    ws_url: String,
    tx: tokio::sync::mpsc::Sender<Review>,
}

impl ReviewListener {
    pub fn new(ws_url: String, tx: tokio::sync::mpsc::Sender<Review>) -> Self {
        Self { ws_url, tx }
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        let mut req = self.ws_url.clone().into_client_request()?;
        req.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_str("graphql-transport-ws")
                .expect("Could not transform header string to header value"),
        );

        info!("Establishing websocket connection to {}", self.ws_url);
        let (ws_stream, resp) = tokio_tungstenite::connect_async(req).await?;
        debug!("Websocket connection established: {:?}", resp);

        let gql = Client::build(ws_stream);

        let mut subscription = gql.subscribe(CreateReviewSubscription::build(())).await?;

        info!("Successfully subscribed to review creation");

        while let Some(msg) = subscription.next().await {
            match msg {
                Ok(msg) => {
                    handle_subscription_message(self, msg).await?;
                }
                Err(err) => {
                    error!("Error while receiving message from subscription: {}", err);
                    return Err(err.into());
                }
            }
        }

        async fn handle_subscription_message(
            listener: &ReviewListener,
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
                listener.tx.send(review).await?;
            } else {
                warn!(
                    "Received message from subscription with unknown data: {:#?}",
                    data
                );
            }

            Ok(())
        }

        Ok(())
    }
}
