use anyhow::Result;
use dioxus::prelude::*;

#[cfg(feature = "server")]
thread_local! {
    static DB: std::sync::LazyLock<rusqlite::Connection> = std::sync::LazyLock::new(|| {
        std::fs::create_dir("hotdogdb").unwrap();
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
pub async fn list_dogs() -> Result<Vec<(usize, String)>> {
    DB.with(|db| {
        Ok(db
            .prepare("SELECT id, url FROM dogs ORDER BY id DESC LIMIT 10")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<(usize, String)>, rusqlite::Error>>()?)
    })
}

#[delete("/api/dogs/{id}")]
pub async fn remove_dog(id: usize) -> Result<()> {
    DB.with(|db| db.execute("DELETE FROM dogs WHERE id = ?1", [id]))?;
    Ok(())
}

#[post("/api/dogs")]
pub async fn save_dog(image: String) -> Result<()> {
    DB.with(|db| db.execute("INSERT INTO dogs (url) VALUES (?1)", [&image]))?;
    Ok(())
}
