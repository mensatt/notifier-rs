use crate::gql::{schema, Review};

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Subscription")]
pub struct CreateReviewSubscription {
    pub review_created: Option<Review>,
}
