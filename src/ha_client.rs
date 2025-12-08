use crate::config::Config;
use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct HaStateResponse {
    state: String,
    // attributes: serde_json::Value, // We can add this if we need more data later
}

#[derive(Clone)]
pub struct HaClient {
    client: Client,
    base_url: String,
    token: String,
}

impl HaClient {
    pub fn new(config: &Config) -> Self {
        HaClient {
            client: Client::new(),
            base_url: config.ha_base_url.clone(),
            token: config.ha_token.clone(),
        }
    }

    pub async fn fetch_sensor_state(&self, entity_id: &str) -> Result<String> {
        let url = format!("{}/api/states/{}", self.base_url, entity_id);

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await?
            .error_for_status()?;

        let json: HaStateResponse = resp.json().await?;
        Ok(json.state)
    }
}
