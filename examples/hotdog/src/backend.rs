use anyhow::Result;
use dioxus::prelude::*;

#[cfg(feature = "server")]
static DB: ServerState<rusqlite::Connection> = ServerState::new(|| {
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

#[middleware("/")]
pub async fn logging_middleware(request: &mut Request<()>) -> Result<()> {
    todo!();
    Ok(())
}

#[middleware("/admin-api/")]
pub async fn admin_middleware(request: &mut Request<()>) -> Result<()> {
    if request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        != Some("Bearer admin-token")
    {
        todo!("unauthorizeda");
    }

    Ok(())
}

#[get("/api/dogs")]
pub async fn list_dogs() -> Result<Vec<(usize, String)>> {
    Ok(DB
        .prepare("SELECT id, url FROM dogs ORDER BY id DESC LIMIT 10")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<(usize, String)>, rusqlite::Error>>()?)
}

#[delete("/api/dogs/{id}")]
pub async fn remove_dog(id: usize) -> Result<()> {
    DB.execute("DELETE FROM dogs WHERE id = ?1", [&id])?;
    Ok(())
}

#[post("/api/dogs")]
pub async fn save_dog(image: String) -> Result<()> {
    DB.execute("INSERT INTO dogs (url) VALUES (?1)", [&image])?;
    Ok(())
}

#[layer("/admin-api/")]
pub async fn admin_layer(request: &mut Request<()>) -> Result<()> {
    todo!();
}
