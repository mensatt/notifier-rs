use crate::gql::{schema, Uuid};

#[derive(cynic::QueryVariables, Debug)]
pub struct UpdateReviewMutationVariables {
    pub approved: bool,
    pub id: Uuid,
}

#[derive(cynic::QueryVariables, Debug)]
pub struct LoginMutationVariables {
    pub email: String,
    pub password: String,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "UpdateReviewMutationVariables")]
pub struct UpdateReviewMutation {
    #[arguments(input: { approved: $approved, id: $id })]
    pub update_review: Review,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Review {
    pub id: Uuid,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Mutation", variables = "LoginMutationVariables")]
pub struct LoginMutation {
    #[arguments(input: { email: $email, password: $password })]
    pub login_user: String,
}
