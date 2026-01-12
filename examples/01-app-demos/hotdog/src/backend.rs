use anyhow::Result;
use dioxus::prelude::*;
use std::sync::Mutex;
use chrono::Local;

#[cfg(feature = "server")]
thread_local! {
    static DB: std::sync::LazyLock<rusqlite::Connection> = std::sync::LazyLock::new(|| {
        std::fs::create_dir("hotdogdb").unwrap();
        let conn = rusqlite::Connection::open("hotdogdb/hotdog.db").expect("Failed to open database");

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS dogs (
                id INTEGER PRIMARY KEY,
                url TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY,
                action TEXT NOT NULL,
                dog_id INTEGER,
                user_id TEXT,
                ip_address TEXT,
                timestamp TEXT NOT NULL,
                result TEXT
            );",
        )
        .unwrap();

        conn
    });
}

// In-memory session store for authorized users (simplified for example)
static AUTHORIZED_USERS: Mutex<std::collections::HashSet<String>> = Mutex::new(std::collections::HashSet::new());

fn is_authorized(user_id: &str) -> bool {
    AUTHORIZED_USERS.lock().unwrap().contains(user_id)
}

fn log_audit(action: &str, dog_id: Option<usize>, user_id: &str, ip_address: &str, result: &str) {
    DB.with(|db| {
        let timestamp = Local::now().to_rfc3339();
        let _ = db.execute(
            "INSERT INTO audit_log (action, dog_id, user_id, ip_address, timestamp, result) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![action, dog_id.map(|id| id as i64), user_id, ip_address, timestamp, result],
        );
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
pub async fn remove_dog(id: usize, user_id: Option<String>) -> Result<()> {
    // Extract user_id and IP (in real app, get from request context)
    let user_id = user_id.unwrap_or_else(|| "anonymous".to_string());
    let ip_address = "unknown"; // In real app, extract from request
    
    // Authorization check: verify user is authorized to delete
    if !is_authorized(&user_id) {
        log_audit("DELETE", Some(id), &user_id, ip_address, "DENIED - User not authorized");
        return Err(anyhow::anyhow!("User {} is not authorized to delete records", user_id));
    }

    // Perform the deletion and log the result
    let result = DB.with(|db| {
        db.execute("DELETE FROM dogs WHERE id = ?1", [id as i64])
    });

    match result {
        Ok(_) => {
            log_audit("DELETE", Some(id), &user_id, ip_address, "SUCCESS");
            Ok(())
        }
        Err(e) => {
            log_audit("DELETE", Some(id), &user_id, ip_address, &format!("FAILED - {}", e));
            Err(e.into())
        }
    }
}

#[post("/api/dogs")]
pub async fn save_dog(image: String) -> Result<()> {
    DB.with(|db| db.execute("INSERT INTO dogs (url) VALUES (?1)", [&image]))?;
    Ok(())
}
