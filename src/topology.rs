use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologyResponse {
    pub model: String,
    pub num_layers: u32,
    pub devices: Vec<Device>,
    pub assignments: Vec<Assignment>,
    pub next_service_map: HashMap<String, String>,
    pub prefetch_windows: HashMap<String, u32>,
    pub solution: Solution,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub name: String,
    pub local_ip: String,
    pub http_port: u16,
    pub grpc_port: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub service: String,
    pub layers: Vec<Vec<u32>>,
    pub next_service: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Solution {
    pub w: Vec<u32>,
    pub n: Vec<u32>,
    pub k: u32,
    pub obj_value: f64,
    pub sets: SolutionSets,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolutionSets {
    #[serde(rename = "M1")]
    pub m1: Vec<u32>,
    #[serde(rename = "M2")]
    pub m2: Vec<u32>,
    #[serde(rename = "M3")]
    pub m3: Vec<u32>,
}

impl TopologyResponse {
    /// Fetch topology from the API
    pub async fn fetch(api_url: &str) -> color_eyre::Result<Self> {
        let url = format!("{}/v1/topology", api_url);
        let response = reqwest::get(&url).await?;
        println!("Fetched topology data from {}", url);
        let topology: TopologyResponse = response.json().await?;
        Ok(topology)
    }

    /// Get device short name (extract first part before dots)
    pub fn device_short_name(device: &str) -> String {
        device.split('.').next().unwrap_or(device).to_string()
    }

    /// Format layer assignments compactly (e.g., [0..11, 12..23, 24..35])
    pub fn format_layers(layers: &[Vec<u32>]) -> String {
        let ranges: Vec<String> = layers
            .iter()
            .map(|range| {
                if range.is_empty() {
                    "[]".to_string()
                } else if range.len() == 1 {
                    range[0].to_string()
                } else {
                    format!("{}..{}", range.first().unwrap(), range.last().unwrap())
                }
            })
            .collect();
        format!("[{}]", ranges.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "run manually"]
    async fn test_fetch_topology() {
        let api_url = "http://localhost:8080";
        let topology = TopologyResponse::fetch(api_url).await;
        assert!(topology.is_ok());
        let topology = topology.unwrap();
        println!("{:#?}", topology);
    }
}
