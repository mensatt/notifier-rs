use crate::settings::Settings;

pub struct ImageClient {
    client: reqwest::Client,
    settings: Settings,
}

impl ImageClient {
    pub fn new(settings: Settings) -> Self {
        let mut default_headers = reqwest::header::HeaderMap::new();
        default_headers.insert(
            "Authorization",
            format!("Bearer {}", settings.image.key)
                .parse()
                .expect("Could  not create image authorization header"),
        );
        Self {
            client: reqwest::Client::builder()
                .default_headers(default_headers)
                .build()
                .expect("Could not create http client for image service"),
            settings,
        }
    }

    pub async fn rotate_image(&self, id: &str, angle: i32) -> anyhow::Result<()> {
        self.client
            .post(format!(
                "{}?id={}&angle={}",
                self.settings.image.rotate_url, id, angle
            ))
            .send()
            .await?;
        Ok(())
    }
}
