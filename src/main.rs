use crate::gql::subscriptions::CreateReviewSubscription;
use cynic::http::ReqwestExt;
use cynic::QueryBuilder;
use cynic::SubscriptionBuilder;
use futures::StreamExt;
use graphql_ws_client::{Client, Connection};
use reqwest::header::HeaderValue;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

mod gql;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting GraphQL subscription client...");

    const WS_URL: &str = "wss://dev-api.mensatt.de/data/graphql";

    let mut req = WS_URL.into_client_request()?;
    req.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_str("graphql-transport-ws").unwrap(),
    );
    println!("Connecting to WebSocket at: {}", WS_URL);

    let (ws_stream, resp) = tokio_tungstenite::connect_async(req).await?;
    println!("WebSocket connection established. Response: {:?}", resp);

    let client = Client::build(ws_stream);
    println!("GraphQL WebSocket client created.");

    println!("Initializing WebSocket connection...");
 
    println!("Starting subscription...");
    let mut sub = client
        .subscribe(CreateReviewSubscription::build(()))
        .await?;
    println!("Subscription started. Waiting for messages...");

    loop {
        tokio::select! {
            Some(msg) = sub.next() => {
                match msg {
                    Ok(data) => println!("Received message: {:?}", data),
                    Err(e) => eprintln!("Error receiving message: {:?}", e),
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("Ctrl+C received, shutting down.");
                break;
            }
        }
    }

    Ok(())
}
