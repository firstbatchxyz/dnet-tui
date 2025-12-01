use dnet_tui::common::ModelInfo;
use dnet_tui::model::{LoadModelView, ModelView};
use dnet_tui::{App, AppView};

// cargo test --package dnet-tui --test test_model_selector -- test_model_selector --exact --nocapture --ignored
#[tokio::test]
#[ignore = "run manually"]
async fn test_model_selector() -> color_eyre::Result<()> {
    fn dummy_model(repo_id: String) -> ModelInfo {
        ModelInfo {
            object: "model".to_string(),
            id: repo_id,
            created: chrono::Utc::now().timestamp_millis() as u128,
            owned_by: "local".to_string(),
        }
    }
    color_eyre::install()?;
    let terminal = ratatui::init();

    let mut app = App::new_at_view(AppView::Model(ModelView::Load(
        LoadModelView::SelectingModel,
    )))?;
    app.available_models = vec![
        dummy_model("gpt-2".to_string()),
        dummy_model("gpt-3".to_string()),
        dummy_model("gpt-4".to_string()),
    ];

    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
