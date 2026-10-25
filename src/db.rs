use sqlx::{Row, SqlitePool};

pub async fn create_db() -> SqlitePool {
    SqlitePool::connect("sqlite://bot.db?mode=rwc")
        .await
        .expect("Не удалось подключится")
}

pub async fn init_db(pool: &SqlitePool) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                text TEXT NOT NULL,
                priority TEXT NOT NULL
            )",
    )
    .execute(pool)
    .await
    .expect("Не удалось создать таблицу");
}

pub async fn add_note(pool: &SqlitePool, name: String, text: String, priority: String) {
    sqlx::query("INSERT INTO notes (name, text, priority) VALUES (?, ?, ?)")
        .bind(name)
        .bind(text)
        .bind(priority)
        .execute(pool)
        .await
        .expect("Ошибка при добавлении");
}

pub async fn delete_note(pool: &SqlitePool, name: &str) {
    sqlx::query("DELETE FROM notes WHERE name = ?")
        .bind(name)
        .execute(pool)
        .await
        .expect("Ошибка удаления");
}

pub async fn select_notes_sorted(pool: &SqlitePool) -> Vec<(String, String, String)> {
    let rows = sqlx::query(
        "SELECT name, text, priority
        FROM notes ORDER BY CASE priority
                        WHEN 'high' THEN 1
                        WHEN 'medium' THEN 2
                        WHEN 'low' THEN 3
                        ELSE 4 END ",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.iter()
        .map(|row| (row.get("name"), row.get("text"), row.get("priority")))
        .collect()
}

pub async fn select_note(pool: &SqlitePool, name: &str) -> Vec<(String, String, String)> {
    let rows = sqlx::query("SELECT name, text, priority FROM notes WHERE name = ?")
        .bind(name)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    rows.iter()
        .map(|row| (row.get("name"), row.get("text"), row.get("priority")))
        .collect()
}
