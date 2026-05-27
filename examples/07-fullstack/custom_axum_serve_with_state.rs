/*
 * Based on my experience, I had no idea how to use Axum's AppState with Dioxus,
 * but now I've figured it out. So I'm going to share what I learned.
 */


#[cfg(feature = "server")]
use {
    axum::{
        extract::State,
        routing::{get, post},
        Json, Router,
    },
    dioxus_cli_config::fullstack_address_or_localhost,
    anyhow::Result,
    dioxus::server::router,
    sqlx::{sqlite::SqlitePoolOptions, SqlitePool},
    tokio::net::TcpListener,
};

use dioxus::prelude::*;

#[cfg(feature = "server")]
#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
}
//You can change the AppState while developing, for example by adding JWT keys.



fn App() -> Element {
    rsx! {
        p {"App"}
    }
}


#[cfg(feature = "server")]
#[tokio::main]
async fn main() -> Result<()> {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite:app.db?mode=rwc")
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT
        )"
    )
    .execute(&pool)
    .await?;

    let app_state = AppState { pool: pool.clone() };

    let api_router = axum::Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/login", post(|| async { "Login" }))
        .with_state(app_state.clone());

    let router = dioxus::server::router(App)
        .nest("/api", api_router);
    // You can use `.nest()` to mount an Axum router inside the Dioxus router
    // You can mount other routers there too, for example /admin/.
    let addr = dioxus_cli_config::fullstack_address_or_localhost();
    // You should use dioxus_cli_config to handle the address

    println!("Starting server on {}", addr);
    println!("🚀 Server running on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, router.into_make_service()).await.unwrap();

    Ok(())
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}
