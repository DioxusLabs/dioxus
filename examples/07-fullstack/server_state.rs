//! This example shows how to use global state to maintain state between server functions.
//!
//!

use dioxus::{fullstack::Lazy, prelude::*, server::axum};
use futures::lock::Mutex;
use std::sync::{
    LazyLock,
    atomic::{AtomicI32, Ordering},
};

#[cfg(feature = "server")]
use sqlx::{Executor, Row};

/*
Option 1:

For simple, synchronous, thread-safe data, we can use statics with atomic types or mutexes.
The `LazyLock` type from the standard library is a great choice for simple, syncronous data
*/
static MESSAGES: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));

#[post("/api/messages")]
async fn add_message() -> Result<()> {
    MESSAGES.lock().await.push("New message".to_string());
    Ok(())
}

#[get("/api/messages")]
async fn read_messages() -> Result<Vec<String>> {
    Ok(MESSAGES.lock().await.clone())
}

/*
Option 2:

For complex async data, we can use the `Lazy` type from Dioxus Fullstack and then initialize it in
our `dioxus::serve` body. `Lazy` types need to be initialized before they are used, but once initialized,
they can be used without `.await` at the callsites.
*/
#[cfg(feature = "server")]
static DATABASE: Lazy<sqlx::SqlitePool> = Lazy::lazy();

#[get("/api/users")]
async fn get_users() -> Result<Vec<String>> {
    let users = DATABASE
        .fetch_all(sqlx::query("SELECT name FROM users"))
        .await?
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<_>>();

    Ok(users)
}

/*
Option 3:

For data that needs to be provided per-request, we can use axum's `Extension` type to provide
data to our app. This is useful for things like request-scoped data or data that needs to be
initialized per-requestz
*/
#[cfg(feature = "server")]
type BroadcastExtension = axum::Extension<tokio::sync::broadcast::Sender<String>>;

#[post("/api/broadcast", ext: BroadcastExtension)]
async fn broadcast_message() -> Result<()> {
    ext.send("New broadcast message".to_string())?;
    Ok(())
}

fn main() {
    // When using `Lazy` items, or axum `Extension`s, we need to initialize them in `dioxus::serve`
    // before launching our app.
    #[cfg(feature = "server")]
    dioxus::serve(app, |mut router| async move {
        use dioxus::server::axum::Extension;
        use sqlx::sqlite::SqlitePoolOptions;

        // For `Lazy` items, we can use the `set` method to initialize them.
        // We can initialize our lazy static state here on the server before using it on our app.
        DATABASE.set(
            SqlitePoolOptions::new()
                .max_connections(5)
                .connect_with("sqlite::memory:".parse().unwrap())
                .await?,
        )?;

        // For axum `Extension`s, we can use the `layer` method to add them to our router.
        router = router.layer(Extension(tokio::sync::broadcast::channel::<String>(16).0));

        Ok(router)
    });

    #[cfg(not(feature = "server"))]
    dioxus::launch(app);
}

fn app() -> Element {
    let users = use_action(get_users);
    let messages = use_action(read_messages);
    let broadcast = use_action(broadcast_message);
    let add = use_action(add_message);

    rsx! {
        div {

        }
    }
}
