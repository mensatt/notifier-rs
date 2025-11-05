use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::gql::mutations::{
    DeleteReviewMutation, DeleteReviewMutationVariables, LoginMutation, LoginMutationVariables,
    UpdateReviewMutation, UpdateReviewMutationVariables,
};
use crate::gql::queries::{RetrieveReviewsQuery, RetrieveReviewsQueryVariables};
use crate::gql::{Review, Uuid};
use crate::settings::Settings;
use cynic::http::ReqwestExt;
use cynic::MutationBuilder;
use cynic::QueryBuilder;
use log::{debug, info};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
// NOTE: There are other fields as well, but we currently don't need them
struct JwtClaims {
    pub exp: u64, // Token expiration timestamp (in s since epoch)
}

struct JwtState {
    token: String,
    expires_at: u64, // Token expiration timestamp (in s since UNIX epoch
}

pub struct MensattGqlClient {
    settings: Settings,
    http_client: reqwest::Client,
    jwt: Arc<RwLock<JwtState>>,
}

impl MensattGqlClient {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            http_client: reqwest::Client::new(),
            jwt: Arc::new(RwLock::new(JwtState {
                token: String::new(),
                expires_at: 0,
            })),
        }
    }

    pub async fn get_jwt(&self) -> anyhow::Result<String> {
        // Get current UNIX timestamp (seconds since epoch in UTC)
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        // Check if current token is still valid, if so return
        {
            let jwt_state = self.jwt.read().unwrap();
            let remaining = jwt_state.expires_at.saturating_sub(now);
            if remaining > self.settings.mensatt.jwt_threshold_secs {
                return Ok(jwt_state.token.clone());
            }
        }

        info!("JWT is close to expiry. Refreshing...");

        // If token has expired, refresh it
        // NOTE: We fetch the new token before acquiring the lock to avoid holding the
        // lock unnecessarily long (e.g. during during the http call)
        // Doing it this way might cause an error later if the lifetime of the JWT is so
        // short that it has expired until we read it - which is very unlikely.
        let new_token = self.refresh_jwt().await?;
        // Not verifying signature is fine, since we only care about expiry timestamp
        let decoded = jsonwebtoken::dangerous::insecure_decode::<JwtClaims>(&new_token)?;
        {
            let mut jwt_state = self.jwt.write().unwrap();
            jwt_state.token = new_token.clone();
            jwt_state.expires_at = decoded.claims.exp;
        }
        Ok(new_token)
    }

    pub async fn refresh_jwt(&self) -> anyhow::Result<String> {
        let login_mutation = LoginMutation::build(LoginMutationVariables {
            email: self.settings.mensatt.user.clone(),
            password: self.settings.mensatt.password.clone(),
        });

        let response = self
            .http_client
            .post(self.settings.graphql.https_url.as_str())
            .run_graphql(login_mutation)
            .await?;

        debug!("Login response: {:#?}", response);

        if response.errors.is_some() {
            return Err(anyhow::anyhow!("Login failed: {:#?}", response.errors));
        }

        let jwt = response
            .data
            .ok_or_else(|| anyhow::anyhow!("Login failed: No response"))?
            .login_user;

        info!("Successfully logged in as {}", jwt);

        {
            let mut jwt_state = self.jwt.write().unwrap();
            jwt_state.token = jwt.clone();
        }

        Ok(jwt)
    }

    pub async fn get_unapproved_reviews(&self) -> anyhow::Result<Vec<Review>> {
        let get_query =
            RetrieveReviewsQuery::build(RetrieveReviewsQueryVariables { approved: false });

        let response = self
            .http_client
            .post(self.settings.graphql.https_url.as_str())
            .bearer_auth(self.get_jwt().await?)
            .run_graphql(get_query)
            .await?;

        debug!("Retrieve reviews response: {:#?}", response);

        if response.errors.is_some() {
            return Err(anyhow::anyhow!(
                "Retrieve reviews failed: {:#?}",
                response.errors
            ));
        }

        if let Some(data) = response.data {
            return Ok(data.reviews);
        }

        Err(anyhow::anyhow!("Got no data when retrieving reviews"))
    }

    pub async fn update_review(&self, id: Uuid, approved: bool) -> anyhow::Result<()> {
        let update_mutation =
            UpdateReviewMutation::build(UpdateReviewMutationVariables { id, approved });

        let response = self
            .http_client
            .post(self.settings.graphql.https_url.as_str())
            .bearer_auth(self.get_jwt().await?)
            .run_graphql(update_mutation)
            .await?;

        debug!("Update review response: {:#?}", response);

        if response.errors.is_some() {
            return Err(anyhow::anyhow!(
                "Update review failed: {:#?}",
                response.errors
            ));
        }

        if let Some(data) = response.data {
            info!(
                "Successfully updated review with id {}",
                data.update_review.id
            );
        }

        Ok(())
    }

    pub async fn delete_review(&self, id: Uuid) -> anyhow::Result<()> {
        let delete_mutation =
            DeleteReviewMutation::build(DeleteReviewMutationVariables { id: id.clone() });

        let response = self
            .http_client
            .post(self.settings.graphql.https_url.as_str())
            .bearer_auth(self.get_jwt().await?)
            .run_graphql(delete_mutation)
            .await?;

        debug!("Delete review response: {:#?}", response);

        if response.errors.is_some() {
            return Err(anyhow::anyhow!(
                "Delete review failed: {:#?}",
                response.errors
            ));
        }

        let was_deleted = response
            .data
            .ok_or_else(|| anyhow::anyhow!("Deleting review '{}' failed: No Response Data", id))?
            .delete_review;

        if !was_deleted {
            return Err(anyhow::anyhow!("Deleting review '{}' failed", id));
        }

        info!("Successfully deleted review with id {}", id);
        Ok(())
    }
}
