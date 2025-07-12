use dioxus::prelude::*;

#[cfg(feature = "server")]
thread_local! {
    static DB: rusqlite::Connection = {
        let conn = rusqlite::Connection::open("hotdogdb/hotdog.db").expect("Failed to open database");

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS dogs (
                id INTEGER PRIMARY KEY,
                url TEXT NOT NULL
            );",
        ).unwrap();

        conn
    };
}

#[server(endpoint = "list_dogs")]
pub async fn list_dogs() -> Result<Vec<(usize, String)>, ServerFnError> {
    let dogs = DB.with(|f| {
        f.prepare("SELECT id, url FROM dogs ORDER BY id DESC LIMIT 10")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    });

    Ok(dogs)
}

#[server(endpoint = "remove_dog")]
pub async fn remove_dog(id: usize) -> Result<(), ServerFnError> {
    DB.with(|f| f.execute("DELETE FROM dogs WHERE id = ?1", [&id]))?;
    Ok(())
}

#[server(endpoint = "save_dog")]
pub async fn save_dog(image: String) -> Result<(), ServerFnError> {
    _ = DB.with(|f| f.execute("INSERT INTO dogs (url) VALUES (?1)", [&image]));
    Ok(())
}
