use dnet_tui::chat::ChatActiveState;
use dnet_tui::views::chat::{ChatMessage, ChatState};
use dnet_tui::{App, AppState};

// cargo test --package dnet-tui --test test_chat -- test_chat_screen --exact --ignored
#[tokio::test]
#[ignore = "run manually"]
async fn test_chat_screen() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let mut chat = ChatActiveState::new("sample-model".into(), 1000);
    chat.add_message(ChatMessage::new_user("How do you prepare a Menemen?"));
    chat.add_message(ChatMessage::new_assistant(r#"
Menemen - one of the classics of Turkish breakfasts - is a delicious, comforting dish made mainly with eggs, tomatoes, peppers, and olive oil (or butter).
Here's a traditional way to prepare it, plus a few regional and personal variations.

## Traditional Menemen Recipe

Ingredients (for 2-3 people):
	•	3 tablespoons olive oil or 1 tablespoon butter (or a mix of both)
	•	2 green peppers (ideally Turkish sivri biber or bell peppers if unavailable), chopped
	•	2 medium ripe tomatoes, peeled and diced (or grated)
	•	3-4 eggs
	•	Salt to taste
	•	Optional: black pepper, red pepper flakes (pul biber)

Instructions:
	1.	Sauté the peppers:
Heat olive oil (or butter) in a wide pan over medium heat. Add the chopped green peppers and sauté until softened and slightly blistered, about 4-5 minutes.
	2.	Add the tomatoes:
Stir in the diced (or grated) tomatoes. Cook until they release their juices and the mixture thickens a bit — usually around 7-10 minutes. You want it juicy, but not watery.
	3.	Season:
Add salt (and optionally black pepper or red pepper flakes) to taste.
	4.	Add the eggs:
Crack the eggs directly over the tomato-pepper mixture.
	•	For a chunkier texture, stir lightly so the whites and yolks remain partly distinct.
	•	For a creamier texture, stir more thoroughly until the eggs are evenly mixed and softly set.
Cook gently, stirring occasionally, until the eggs reach your desired consistency — typically 2-4 minutes. Avoid overcooking; Menemen should stay moist and silky.
	5.	Serve immediately:
Serve hot, straight from the pan, with plenty of fresh bread (preferably simit, pide, or village bread) for dipping.

## Optional Add-Ins (Regional & Personal Twists)
	•	White cheese or feta: crumble in just before the eggs set for a creamy, salty touch.
	•	Sucuk (Turkish sausage): fry slices before adding peppers for a heartier version.
	•	Onion: a divisive addition! Some regions (especially Aegean) include finely chopped onions sautéed first; others insist real Menemen never has onion.

Would you like me to show a one-pan minimalistic "village-style" version (just eggs, tomatoes, and olive oil) or a restaurant-style version with cheese and sucuk next?
"#));

    let mut app = App::new_with_state(AppState::Chat(ChatState::Active))?;
    app.chat = chat;
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
