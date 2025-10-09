//! This example shows how to use global state to maintain state between server functions.

use dioxus::prelude::*;

#[cfg(feature = "server")]
use {
    dioxus::fullstack::Lazy,
    dioxus::fullstack::axum,
    futures::lock::Mutex,
    sqlx::{Executor, Row},
    std::sync::LazyLock,
};

/*
Option 1:

For simple, synchronous, thread-safe data, we can use statics with atomic types or mutexes.
The `LazyLock` type from the standard library is a great choice for simple, synchronous data
*/
#[cfg(feature = "server")]
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

For complex async data, we can use the `Lazy` type from Dioxus Fullstack. The `Lazy` type provides
an interface like `once_cell::Lazy` but supports async initialization. When reading the value from
a `Lazy<T>`, the value will be initialized synchronously, blocking the current task until the value is ready.

Alternatively, you can create a `Lazy<T>` with `Lazy::lazy` and then initialize it later with
`Lazy::initialize`.
*/
#[cfg(feature = "server")]
static DATABASE: Lazy<sqlx::SqlitePool> = Lazy::new(|| async move {
    use sqlx::sqlite::SqlitePoolOptions;
    dioxus::Ok(
        SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with("sqlite::memory:".parse().unwrap())
            .await?,
    )
});

/// When using the `Lazy<T>` type, it implements `Deref<Target = T>`, so you can use it like a normal reference.
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
    #[cfg(not(feature = "server"))]
    dioxus::launch(app);

    // When using `Lazy` items, or axum `Extension`s, we need to initialize them in `dioxus::serve`
    // before launching our app.
    #[cfg(feature = "server")]
    dioxus::serve(|| async move {
        use dioxus::server::axum::Extension;

        // For axum `Extension`s, we can use the `layer` method to add them to our router.
        let router = dioxus::server::router(app)
            .layer(Extension(tokio::sync::broadcast::channel::<String>(16).0));

        Ok(router)
    });
}

fn app() -> Element {
    let mut users = use_action(get_users);
    let mut messages = use_action(read_messages);
    let mut broadcast = use_action(broadcast_message);
    let mut add = use_action(add_message);

    rsx! {
        div {
            button { onclick: move |_| users.call(), "Get Users" }
            pre { "{users.result():?}" }
            button { onclick: move |_| messages.call(), "Get Messages" }
            pre { "{messages.result():?}" }
            button { onclick: move |_| broadcast.call(), "Broadcast Message" }
            pre { "{broadcast.result():?}" }
            button { onclick: move |_| add.call(), "Add Message" }
            pre { "{add.result():?}" }
        }
    }
}
