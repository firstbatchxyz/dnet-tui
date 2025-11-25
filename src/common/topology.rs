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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelInfo {
    /// Creation timestamp.
    pub created: u128,
    /// Repo ID, can be used to load the model from HuggingFace or other sources.
    pub id: String,
    /// The object type (OpenAI compatibility), usually "model".
    pub object: String,
    /// The owner of the model, usually `local` for dnet.
    pub owned_by: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssignmentInfo {
    pub instance: String,
    pub layers: Vec<Vec<u32>>,
    pub next_instance: String,
    pub window_size: u32,
    pub residency_size: u32,
}
