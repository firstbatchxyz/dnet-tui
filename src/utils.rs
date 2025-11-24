use std::time::Duration;

/// Offset in milliseconds for sliding effect, the higher the slower.
const SLIDING_TEXT_OFFSET_MILLIS: usize = 500;

/// Get sliding window of text based on elapsed time
pub fn get_sliding_text(duration: Duration, full_text: &str, window_size: usize) -> String {
    if full_text.len() <= window_size {
        // return full text if window is larger than text
        full_text.to_string()
    } else {
        // calculate offset based on elapsed milliseconds
        let elapsed_millis = duration.as_millis() as usize;
        let offset = (elapsed_millis / SLIDING_TEXT_OFFSET_MILLIS) % full_text.len();

        // create sliding window by cycling through the text
        format!("{} {}", &full_text[offset..], &full_text[..offset])
            .chars()
            .take(window_size)
            .collect()
    }
}

/// A wrapper around model `config.json` on HuggingFace.
///
/// It is not a strict type because the config may change from model to model.
/// Instead we provide getters for the fields that we are interested in.
pub struct ModelConfig(serde_json::Value);

impl ModelConfig {
    /// Returns the number of layers, tries to read the following:
    ///
    /// - num_hidden_layers
    /// - num_layers
    /// - num_hidden
    pub fn num_layers(&self) -> Option<u64> {
        for key in ["num_hidden_layers", "num_layers", "num_hidden"] {
            if let Some(value) = self.0.get(key) {
                if let Some(n) = value.as_u64() {
                    return Some(n);
                }
            }
        }
        None
    }
    /// Fetches the model config from HuggingFace (via `raw/main/config.json`).
    pub async fn get_model_config(repo_id: &str) -> color_eyre::Result<Self> {
        let url = format!("https://huggingface.co/{repo_id}/raw/main/config.json");
        let res = reqwest::get(url).await?;
        let json: serde_json::Value = res.json().await?;
        Ok(ModelConfig(json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slider() {
        // ensure text slides correctly w.r.t duration
        let text = "abc";
        let x = Duration::from_millis(SLIDING_TEXT_OFFSET_MILLIS as u64);

        // smaller window than text length
        // (should cycle-back at text length)
        assert_eq!(get_sliding_text(x * 0, text, 2), "ab");
        assert_eq!(get_sliding_text(x * 1, text, 2), "bc");
        assert_eq!(get_sliding_text(x * 2, text, 2), "c ");
        assert_eq!(get_sliding_text(x * 3, text, 2), "ab");

        // exact window size equals text length (should not slide at all)
        assert_eq!(get_sliding_text(x * 0, text, 3), "abc");
        assert_eq!(get_sliding_text(x * 1, text, 3), "abc");
        assert_eq!(get_sliding_text(x * 2, text, 3), "abc");

        // larger window than text length (should not slide at all)
        assert_eq!(get_sliding_text(x * 0, text, 5), "abc");
        assert_eq!(get_sliding_text(x * 1, text, 5), "abc");
        assert_eq!(get_sliding_text(x * 2, text, 5), "abc");
        assert_eq!(get_sliding_text(x * 3, text, 5), "abc");
    }

    #[tokio::test]
    async fn test_model_config() {
        let config = ModelConfig::get_model_config("Qwen/Qwen3-32B-MLX-bf16")
            .await
            .unwrap();
        assert_eq!(config.num_layers(), Some(64));
    }
}
