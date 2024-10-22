use crate::gql::schema;
use std::fmt::{Display, Formatter};

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Subscription")]
pub struct CreateReviewSubscription {
    pub review_created: Option<Review>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Review {
    pub id: Uuid,
    pub occurrence: Occurrence,
    pub display_name: Option<String>,
    pub stars: i32,
    pub text: Option<String>,
    pub created_at: Timestamp,
    pub images: Vec<Image>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Occurrence {
    pub id: Uuid,
    pub dish: Dish,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Image {
    pub id: Uuid,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Dish {
    pub name_de: String,
}

#[derive(cynic::Scalar, Debug, Clone)]
pub struct Timestamp(pub String);

#[derive(cynic::Scalar, Debug, Clone)]
#[cynic(graphql_type = "UUID")]
pub struct Uuid(pub String);

// TODO: Is there a better way for this?
impl Display for Uuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
