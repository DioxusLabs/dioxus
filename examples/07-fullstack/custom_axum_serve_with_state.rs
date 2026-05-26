/*
 * Based on my experience, I had no idea how to use Axum's AppState with Dioxus,
 * but now I've figured it out. So I'm going to share what I learned.
 */

#[cfg(feature = "server")]
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
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
        .route("/login", post(login_handler))
        .with_state(app_state.clone());

    let router = dioxus::server::router(App)
        .nest("/api", api_router);
    // You can use `.nest()` to mount an Axum router inside the Dioxus router

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
