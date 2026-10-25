use crate::db::*;
use crate::state;
use sqlx::SqlitePool;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::types::ParseMode;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    utils::command::BotCommands,
};

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
type MyDialogue = Dialogue<state::State, InMemStorage<state::State>>;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    Start,
}

pub async fn start_handler(bot: Bot, message: Message, command: Command) -> HandlerResult {
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("📂Показать заметки📂", "show_notes"),
        InlineKeyboardButton::callback("️➕Добавить заметку➕", "add_note"),
    ]]);
    match command {
        Command::Start => {
            bot.send_message(message.chat.id, "Приветствую! Это бот для хранения заметок, лично для тебя римуру) А ну еще гусь черт").reply_markup(keyboard).await?;
        }
    }
    Ok(())
}

pub async fn callback_start_handler(
    bot: Bot,
    callback: CallbackQuery,
    dialogue: MyDialogue,
    pool: SqlitePool,
) -> HandlerResult {
    if let Some(data) = callback.data {
        match data.as_str() {
            "show_notes" => {
                bot.answer_callback_query(callback.id).await?;
                let notes = select_notes_sorted(&pool).await;
                if notes.is_empty() {
                    bot.send_message(
                        callback.message.unwrap().chat().id,
                        "У тебя пока нету заметок броу",
                    )
                    .await?;
                    return Ok(());
                }
                let mut keyboard = vec![];
                for (name, _, priority) in notes {
                    let button_text = format!("[{priority}] {name}");
                    let button_callback = format!("note_{name}");

                    keyboard.push(vec![InlineKeyboardButton::callback(
                        button_text,
                        button_callback,
                    )]);
                }
                keyboard.push(vec![InlineKeyboardButton::callback("Назад🔙", "back")]);

                bot.send_message(callback.message.unwrap().chat().id, "Ваши заметки:")
                    .reply_markup(InlineKeyboardMarkup::new(keyboard))
                    .await?;
            }
            "add_note" => {
                bot.answer_callback_query(callback.id).await?;
                bot.send_message(
                    callback.message.unwrap().chat().id,
                    "1/3, введите название для заметки:",
                )
                .await?;
                dialogue.update(state::State::Title).await?;
            }
            _ => {}
        };
    }
    Ok(())
}

pub async fn receive_title(bot: Bot, message: Message, dialogue: MyDialogue) -> HandlerResult {
    let Some(title) = message.text() else {
        bot.send_message(message.chat.id, "Нормально введи бля")
            .await?;
        return Ok(());
    };

    bot.send_message(message.chat.id, "2/3, введите текст заметки:")
        .await?;

    dialogue
        .update(state::State::Text {
            title: title.to_string(),
        })
        .await?;
    Ok(())
}

pub async fn receive_text(
    bot: Bot,
    message: Message,
    dialogue: MyDialogue,
    title: String,
) -> HandlerResult {
    let Some(text) = message.text() else {
        bot.send_message(message.chat.id, "Нормально введи бля")
            .await?;
        return Ok(());
    };

    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔴Высокий🔴", "high")],
        vec![InlineKeyboardButton::callback("🟡Средний🟡", "medium")],
        vec![InlineKeyboardButton::callback("🟢Низкий 🟢", "low")],
    ]);

    bot.send_message(
        message.chat.id,
        format!("3/3, выберите приоритет для заметки: {title}\n{text}"),
    )
    .reply_markup(keyboard)
    .await?;

    dialogue
        .update(state::State::Priority {
            title,
            text: text.to_string(),
        })
        .await?;

    Ok(())
}

pub async fn receive_priority(
    bot: Bot,
    callback: CallbackQuery,
    dialogue: MyDialogue,
    (title, text): (String, String),
    pool: SqlitePool,
) -> HandlerResult {
    bot.answer_callback_query(callback.id).await?;

    let Some(msg) = callback.message else {
        return Ok(());
    };

    let Some(data) = callback.data else {
        return Ok(());
    };

    add_note(&pool, title, text, data).await;
    let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "Назад🔙",
        "back",
    )]]);

    bot.edit_message_text(msg.chat().id, msg.id(), "Ваша заметка успешно добавлена!")
        .reply_markup(keyboard)
        .await?;

    dialogue.exit().await?;
    Ok(())
}

pub async fn callback_show_note(
    bot: Bot,
    callback: CallbackQuery,
    pool: SqlitePool,
) -> HandlerResult {
    bot.answer_callback_query(callback.id).await?;

    let Some(data) = callback.data else {
        return Ok(());
    };
    let note_name = data.strip_prefix("note_").unwrap_or(&data);

    let note = select_note(&pool, note_name).await;
    let Some((name, text, priority)) = note.get(0) else {
        return Ok(());
    };
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "🗑️Удалить заметку🗑️",
            format!("del_{name}"),
        )],
        vec![InlineKeyboardButton::callback("Назад🔙", "back")],
    ]);

    bot.send_message(
        callback.message.unwrap().chat().id,
        format!("📌 *{name}*\nПриоритет: `{priority}`\n\nТекст заметки: {text}"),
    )
    .reply_markup(keyboard)
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    Ok(())
}

pub async fn remove_note(bot: Bot, callback: CallbackQuery, pool: SqlitePool) -> HandlerResult {
    bot.answer_callback_query(callback.id).await?;

    let Some(data) = callback.data else {
        return Ok(());
    };

    let note_name = data.strip_prefix("del_").unwrap_or(&data);

    delete_note(&pool, note_name).await;

    let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "Назад🔙",
        "back",
    )]]);

    bot.send_message(callback.message.unwrap().chat().id, "🗑️Заметка удалена🗑️")
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

pub async fn back_handle(bot: Bot, callback: CallbackQuery) -> HandlerResult {
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("📂Показать заметки📂", "show_notes"),
        InlineKeyboardButton::callback("️➕Добавить заметку➕", "add_note"),
    ]]);

    bot.send_message(
        callback.message.unwrap().chat().id,
        "Приветствую! Это бот для хранения заметок, лично для тебя римуру) А ну еще гусь черт",
    )
    .reply_markup(keyboard)
    .await?;

    Ok(())
}
