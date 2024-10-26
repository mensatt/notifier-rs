use crate::gql::mutations::{
    DeleteReviewMutation, DeleteReviewMutationVariables, LoginMutation, LoginMutationVariables,
    UpdateReviewMutation, UpdateReviewMutationVariables,
};
use crate::gql::Uuid;
use crate::settings::Settings;
use cynic::http::ReqwestExt;
use cynic::MutationBuilder;
use log::{debug, info};
use reqwest::header::HeaderMap;

pub struct MensattGqlClient {
    settings: Settings,
    http_client: reqwest::Client,
}

impl MensattGqlClient {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn login(&mut self) -> anyhow::Result<()> {
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

        if let Some(data) = response.data {
            info!("Successfully logged in as {}", data.login_user);

            let mut headers = HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", data.login_user).parse()?,
            );

            // I *really* do not like this, but it is not possible to insert default headers
            // after the client has been created
            self.http_client = reqwest::Client::builder()
                .default_headers(headers)
                .build()?;
        } else {
            return Err(anyhow::anyhow!("Login failed: No response"));
        }

        Ok(())
    }

    pub async fn update_review(&self, id: Uuid, approved: bool) -> anyhow::Result<()> {
        let update_mutation =
            UpdateReviewMutation::build(UpdateReviewMutationVariables { id, approved });

        let response = self
            .http_client
            .post(self.settings.graphql.https_url.as_str())
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
        let delete_mutation = DeleteReviewMutation::build(DeleteReviewMutationVariables { id });

        let response = self
            .http_client
            .post(self.settings.graphql.https_url.as_str())
            .run_graphql(delete_mutation)
            .await?;

        debug!("Delete review response: {:#?}", response);

        if response.errors.is_some() {
            return Err(anyhow::anyhow!(
                "Delete review failed: {:#?}",
                response.errors
            ));
        }

        if let Some(data) = response.data {
            info!(
                "Successfully deleted review with id {}",
                data.delete_review.id
            );
        }

        Ok(())
    }
}
