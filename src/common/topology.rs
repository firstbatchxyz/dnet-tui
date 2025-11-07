use crate::config::KVBits;

use super::DeviceProperties;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologyInfo {
    /// The loaded model.
    ///
    /// If the topology was prepared but the model is later unloaded, this is `None`.
    pub model: Option<String>,

    /// Number of layers in the model.
    pub num_layers: u32,
    /// The list of discovered **shards**.
    pub devices: Vec<DeviceProperties>,
    /// Assignments of layers to instances.
    ///
    /// Each [`Assignment`] describes which layers are assigned to which instancee,
    /// and the `instance` field corresponds to the `instance` field in [`DeviceProperties`].
    pub assignments: Vec<AssignmentInfo>,
    // can be anything
    // pub solution: Option<serde_json::Value>,
    pub kv_bits: KVBits,
}

impl TopologyInfo {
    /// Fetch topology from the API
    pub async fn fetch(api_url: &str) -> color_eyre::Result<TopologyInfo> {
        let url = format!("{}/v1/topology", api_url);
        let response = reqwest::get(&url).await?;

        // Get the response text first, regardless of status
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssignmentInfo {
    pub instance: String,
    pub layers: Vec<Vec<u32>>,
    pub next_instance: String,
    pub window_size: u32,
    pub residency_size: u32,
}
