use crate::gql::schema;

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query")]
pub struct RetrieveReviewsQuery {
    #[arguments(filter: { approved: false })]
    pub reviews: Vec<Review>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Review {
    pub id: Uuid,
    pub occurrence: Occurrence,
    pub display_name: Option<String>,
    pub stars: i32,
    pub text: Option<String>,
    pub updated_at: Timestamp,
    pub images: Vec<Image>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Occurrence {
    pub dish: Dish,
    pub id: Uuid,
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
