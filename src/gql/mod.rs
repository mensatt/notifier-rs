use std::fmt::{Display, Formatter};

pub mod client;
pub mod listener;
mod mutations;
pub mod queries;
pub mod subscriptions;

#[cynic::schema("mensatt")]
mod schema {}

#[derive(cynic::Scalar, Debug, Clone)]
#[cynic(graphql_type = "UUID")]
pub struct Uuid(pub String);

// TODO: Is there a better way for this?
impl Display for Uuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
