mod db;
mod handlers;
mod state;

use db::*;
use dotenvy::dotenv;
use std::env;
use teloxide::dispatching::{UpdateHandler, dialogue::InMemStorage};
use teloxide::prelude::*;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("BOT_TOKEN").expect("Переменная окружения BOT_TOKEN не задана");
    let bot = Bot::new(token);

    let pool = create_db().await;
    init_db(&pool).await;

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![pool, InMemStorage::<state::State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    let command_branch = teloxide::filter_command::<handlers::Command, _>()
        .branch(dptree::case![handlers::Command::Start].endpoint(handlers::start_handler));

    let message_branch = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<state::State>, state::State>()
        .branch(command_branch)
        .branch(dptree::case![state::State::Title].endpoint(handlers::receive_title))
        .branch(dptree::case![state::State::Text { title }].endpoint(handlers::receive_text));

    let callback_branch = Update::filter_callback_query()
        .enter_dialogue::<CallbackQuery, InMemStorage<state::State>, state::State>()
        .branch(
            dptree::filter(|callback: CallbackQuery| {
                callback.data.as_deref().unwrap_or("").starts_with("note_")
            })
            .endpoint(handlers::callback_show_note),
        )
        .branch(
            dptree::filter(|callback: CallbackQuery| {
                callback.data.as_deref().unwrap_or("").starts_with("del_")
            })
            .endpoint(handlers::remove_note),
        )
        .branch(
            dptree::filter(|callback: CallbackQuery| callback.data.as_deref() == Some("back"))
                .endpoint(handlers::back_handle),
        )
        .branch(dptree::case![state::State::Start].endpoint(handlers::callback_start_handler))
        .branch(
            dptree::case![state::State::Priority { title, text }]
                .endpoint(handlers::receive_priority),
        );

    dptree::entry()
        .branch(message_branch)
        .branch(callback_branch)
}
