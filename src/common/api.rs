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
            base_url: format!("http://{host}:{port}"),
        }
    }

    pub async fn is_healthy(&self) -> color_eyre::Result<bool> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }

    pub async fn get_models(&self) -> color_eyre::Result<Vec<ModelInfo>> {
        #[derive(Deserialize, Serialize)]
        pub struct ListModelsResponse {
            pub object: String,
            pub data: Vec<ModelInfo>,
        }

        let url = format!("{}/v1/models", self.base_url);
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            color_eyre::eyre::bail!(
                "Failed to get models: ({}) {}",
                response.status(),
                response.text().await?
            );
        }

        let models: ListModelsResponse = response.json().await?;
        Ok(models.data)
    }

    pub async fn get_topology(&self) -> color_eyre::Result<Option<TopologyInfo>> {
        let url = format!("{}/v1/topology", self.base_url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let topology = response
                .json::<TopologyInfo>()
                .await
                .map_err(|e| color_eyre::eyre::eyre!("Failed to parse topology response: {}", e))?;
            Ok(Some(topology))
        } else if response.status() == reqwest::StatusCode::BAD_REQUEST {
            Ok(None)
        } else {
            color_eyre::eyre::bail!(
                "Failed to get topology: ({}) {}",
                response.status(),
                response.text().await?
            )
        }
    }

    pub async fn get_devices(&self) -> color_eyre::Result<HashMap<String, DeviceProperties>> {
        #[derive(Debug, Clone, Deserialize)]
        pub struct DevicesResponse {
            pub devices: HashMap<String, DeviceProperties>,
        }
        let url = format!("{}/v1/devices", self.base_url);
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            color_eyre::eyre::bail!("Failed to get devices: {}", response.text().await?);
        }

        let devices_response: DevicesResponse = response.json().await?;
        Ok(devices_response.devices)
    }

    pub async fn load_model(&self, model: &str) -> color_eyre::Result<LoadModelResponse> {
        let url = format!("{}/v1/load_model", self.base_url);
        let body = serde_json::json!({"model": model});

        let response = self.client.post(&url).json(&body).send().await?;
        if !response.status().is_success() {
            color_eyre::eyre::bail!("Failed to load model: {}", response.text().await?)
        }

        let load_response: LoadModelResponse = response.json().await?;
        Ok(load_response)
    }

    pub async fn unload_model(&self) -> color_eyre::Result<()> {
        let url = format!("{}/v1/unload_model", self.base_url);

        let response = self.client.post(&url).send().await?;
        if response.status().is_success() {
            Ok(())
        } else {
            color_eyre::eyre::bail!("Failed to unload model: {}", response.text().await?)
        }
    }

    pub async fn prepare_topology(
        &self,
        config: &crate::Config,
        model: &str,
    ) -> color_eyre::Result<TopologyInfo> {
        let url = format!("{}/v1/prepare_topology", self.base_url);
        let body = serde_json::json!({
            "model": model.to_string(),
            "kv_bits": config.kv_bits,
            "seq_len": config.seq_len,
            "max_batch_exp": config.max_batch_exp,
        });

        let response = self.client.post(&url).json(&body).send().await?;
        if !response.status().is_success() {
            color_eyre::eyre::bail!("Failed to prepare topology: {}", response.text().await?);
        }

        let topology: TopologyInfo = response.json().await?;
        Ok(topology)
    }

    pub async fn prepare_topology_manual(
        &self,
        config: &crate::Config,
        model: &str,
        num_layers: u32,
        devices: Vec<crate::common::DeviceProperties>,
        assignments: Vec<crate::common::AssignmentInfo>,
    ) -> color_eyre::Result<TopologyInfo> {
        let url = format!("{}/v1/prepare_topology_manual", self.base_url);
        let body = serde_json::json!({
            "model": model.to_string(),
            "devices": devices,
            "assignments": assignments,
            "num_layers": num_layers,
            "kv_bits": config.kv_bits,
            "seq_len": config.seq_len,
            "max_batch_exp": config.max_batch_exp,
        });

        let response = self.client.post(&url).json(&body).send().await?;
        if !response.status().is_success() {
            color_eyre::eyre::bail!(
                "Failed to prepare manual topology: {}",
                response.text().await?
            );
        }

        let topology: TopologyInfo = response.json().await?;
        Ok(topology)
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadModelResponse {
    /// Model name
    pub model: String,
    /// Whether all shards loaded successfully
    pub success: bool,
    /// Status of each shard
    pub shard_statuses: Vec<ShardLoadStatus>,
    /// Overall status or error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShardLoadStatus {
    /// Shard name
    pub instance: String,
    /// Whether loading succeeded
    pub success: bool,
    /// Layers successfully loaded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers_loaded: Option<Vec<u32>>,
    /// Status or error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
