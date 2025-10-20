use dioxus::prelude::*;
use miette::IntoDiagnostic;

#[cfg(feature = "server")]
thread_local! {
    static DB: std::sync::LazyLock<rusqlite::Connection> = std::sync::LazyLock::new(|| {
        let conn = rusqlite::Connection::open("hotdogdb/hotdog.db").expect("Failed to open database");

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS dogs (
                id INTEGER PRIMARY KEY,
                url TEXT NOT NULL
            );",
        )
        .unwrap();

        conn
    });
}

#[get("/api/dogs")]
pub async fn list_dogs() -> anyhow::Result<Vec<(usize, String)>> {
    DB.with(|db| {
        Ok(db
            .prepare("SELECT id, url FROM dogs ORDER BY id DESC LIMIT 10")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<(usize, String)>, rusqlite::Error>>()?)
    })
}

#[delete("/api/dogs/{id}")]
pub async fn remove_dog(id: usize) -> anyhow::Result<()> {
    DB.with(|db| db.execute("DELETE FROM dogs WHERE id = ?1", [id]))?;
    Ok(())
}

#[post("/api/dogs")]
pub async fn save_dog(image: String) -> miette::Result<()> {
    DB.with(|db| db.execute("INSERT INTO dogs (url) VALUES (?1)", [&image]))
        .into_diagnostic()?;
    Ok(())
}
