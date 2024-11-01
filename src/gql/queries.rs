use crate::gql::{schema, Review};
#[derive(cynic::QueryVariables, Debug)]
pub struct RetrieveReviewsQueryVariables {
    pub approved: bool,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "RetrieveReviewsQueryVariables")]
pub struct RetrieveReviewsQuery {
    #[arguments(filter: { approved: $approved })]
    pub reviews: Vec<Review>,
}
