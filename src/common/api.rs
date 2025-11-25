use std::collections::HashMap;

use crate::common::{DeviceProperties, ModelInfo, TopologyInfo};

use serde::{Deserialize, Serialize};
#[derive(Debug)]
pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl Default for ApiClient {
    fn default() -> Self {
        ApiClient::new("localhost", 8080)
    }
}

impl ApiClient {
    pub fn new(host: &str, port: u16) -> Self {
        ApiClient {
            client: reqwest::Client::new(),
            base_url: format!("http://{}:{}", host, port),
        }
    }

    pub async fn get_models(&self) -> color_eyre::Result<Vec<ModelInfo>> {
        #[derive(Deserialize, Serialize)]
        pub struct ListModelsResponse {
            pub object: String,
            pub data: Vec<ModelInfo>,
        }

        let url = format!("{}/v1/models", self.base_url);
        let response = self.client.get(&url).send().await?;
        let models: ListModelsResponse = response.json().await?;
        Ok(models.data)
    }

    pub async fn is_healthy(&self) -> color_eyre::Result<bool> {
        let url = format!("{}/v1/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }

    /// Fetch topology from the API
    pub async fn get_topology(&self) -> color_eyre::Result<TopologyInfo> {
        let url = format!("{}/v1/topology", self.base_url);
        let response = self.client.get(&url).send().await?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await?;
            Err(color_eyre::eyre::eyre!(
                "API returned error: {} {}",
                status.as_u16(),
                text
            ))
        } else {
            // Try to parse as successful topology response
            response
                .json()
                .await
                .map_err(|e| color_eyre::eyre::eyre!("Failed to parse topology response: {}", e))
        }
    }

    /// Fetch devices from the API
    pub async fn get_devices(&self) -> color_eyre::Result<HashMap<String, DeviceProperties>> {
        /// The response from the `/v1/devices` endpoint.
        #[derive(Debug, Clone, Deserialize)]
        pub struct DevicesResponse {
            pub devices: HashMap<String, DeviceProperties>,
        }

        let url = format!("{}/v1/devices", self.base_url);
        let response = reqwest::get(&url).await?;

        if !response.status().is_success() {
            color_eyre::eyre::bail!("API returned error: {}", response.status());
        }

        let devices_response: DevicesResponse = response.json().await?;
        Ok(devices_response.devices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "run manually"]
    async fn test_get_models() {
        let api = ApiClient::default();
        let result = api.get_models().await;
        assert!(result.is_ok());

        let models = result.unwrap();
        println!("Retrieved {} models:", models.len());
        for model in models {
            println!("{}", model.id);
        }
    }

    #[tokio::test]
    #[ignore = "run manually"]
    async fn test_get_topology() {
        let api = ApiClient::default();
        let topology = api.get_topology().await;
        println!("{:#?}", topology);
        assert!(topology.is_ok());
    }
}
