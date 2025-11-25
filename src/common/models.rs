use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListModelsResponse {
    pub object: String,
    pub data: Vec<ModelInfo>,
}

pub async fn get_models_from_api(api_url: &str) -> color_eyre::Result<Vec<ModelInfo>> {
    let url = format!("{}/v1/models", api_url);
    let response = reqwest::get(url).await?;
    let models: ListModelsResponse = response.json().await?;
    Ok(models.data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_models_from_api() {
        let api_url = "http://localhost:8080";
        let result = get_models_from_api(api_url).await;
        assert!(result.is_ok());

        let models = result.unwrap();
        println!("Retrieved {} models:", models.len());
        for model in models {
            println!("{}", model.id);
        }
    }
}
