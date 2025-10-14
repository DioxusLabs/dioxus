//! This example showcases how to use the `axum-session-auth` crate with Dioxus fullstack.
//! We add the `auth::Session` extractor to our server functions to get access to the current user session.
//!
//! To initialize the axum router, we use `dioxus::serve` to spawn a custom axum server that creates
//! our database, session store, and authentication layer.
//!
//! The `.serve_dioxus_application` method is used to mount our Dioxus app as a fallback service to
//! handle HTML rendering and static assets.
//!
//! We easily share the "permissions" between the server and client by using a `HashSet<String>`
//! which is serialized to/from JSON automatically by the server function system.

use std::collections::HashSet;

use dioxus::prelude::*;

#[cfg(feature = "server")]
mod auth;

fn main() {
    // On the client, we simply launch the app as normal, taking over the main thread
    #[cfg(not(feature = "server"))]
    dioxus::launch(app);

    // On the server, we can use `dioxus::serve` to create a server that serves our app.
    //
    // The `serve` function takes a closure that returns a `Future` which resolves to an `axum::Router`.
    //
    // We return a `Router` such that dioxus sets up logging, hot-reloading, devtools, and wires up the
    // IP and PORT environment variables to our server.
    #[cfg(feature = "server")]
    dioxus::serve(|| async {
        use crate::auth::*;
        use axum_session::{SessionConfig, SessionLayer, SessionStore};
        use axum_session_auth::AuthConfig;
        use axum_session_sqlx::SessionSqlitePool;
        use sqlx::{sqlite::SqlitePoolOptions, Executor};

        // Create an in-memory SQLite database and set up our tables
        let db = SqlitePoolOptions::new()
            .max_connections(20)
            .connect_with("sqlite::memory:".parse()?)
            .await?;

        // Create the tables (sessions, users)
        db.execute(r#"CREATE TABLE IF NOT EXISTS users ( "id" INTEGER PRIMARY KEY, "anonymous" BOOLEAN NOT NULL, "username" VARCHAR(256) NOT NULL )"#,)
            .await?;
        db.execute(r#"CREATE TABLE IF NOT EXISTS user_permissions ( "user_id" INTEGER NOT NULL, "token" VARCHAR(256) NOT NULL)"#,)
            .await?;

        // Insert in some test data for two users (one anonymous, one normal)
        db.execute(r#"INSERT INTO users (id, anonymous, username) SELECT 1, true, 'Guest' ON CONFLICT(id) DO UPDATE SET anonymous = EXCLUDED.anonymous, username = EXCLUDED.username"#,)
            .await?;
        db.execute(r#"INSERT INTO users (id, anonymous, username) SELECT 2, false, 'Test' ON CONFLICT(id) DO UPDATE SET anonymous = EXCLUDED.anonymous, username = EXCLUDED.username"#,)
            .await?;

        // Make sure our test user has the ability to view categories
        db.execute(r#"INSERT INTO user_permissions (user_id, token) SELECT 2, 'Category::View'"#)
            .await?;

        // Create an axum router that dioxus will attach the app to
        Ok(dioxus::server::router(app)
            .layer(
                AuthLayer::new(Some(db.clone()))
                    .with_config(AuthConfig::<i64>::default().with_anonymous_user_id(Some(1))),
            )
            .layer(SessionLayer::new(
                SessionStore::<SessionSqlitePool>::new(
                    Some(db.into()),
                    SessionConfig::default().with_table_name("test_table"),
                )
                .await?,
            )))
    });
}

/// The UI for our app - is just a few buttons to call our server functions and display the results.
fn app() -> Element {
    let mut login = use_action(login);
    let mut user_name = use_action(get_user_name);
    let mut permissions = use_action(get_permissions);
    let mut logout = use_action(logout);

    let fetch_new = move |_| async move {
        user_name.call().await;
        permissions.call().await;
    };

    rsx! {
        div {
            button {
                onclick: move |_| async move {
                    login.call().await;
                },
                "Login Test User"
            }
            button {
                onclick: move |_| async move {
                    logout.call().await;
                },
                "Logout"
            }
            button {
                onclick: fetch_new,
                "Fetch User Info"
            }

            pre { "Logged in: {login.value():?}" }
            pre { "User name: {user_name.value():?}" }
            pre { "Permissions: {permissions.value():?}" }
        }
    }
}

/// We use the `auth::Session` extractor to get access to the current user session.
/// This lets us modify the user session, log in/out, and access the current user.
#[post("/api/user/login", auth: auth::Session)]
pub async fn login() -> Result<()> {
    auth.login_user(2);
    Ok(())
}

/// Just like `login`, but this time we log out the user.
#[post("/api/user/logout", auth: auth::Session)]
pub async fn logout() -> Result<()> {
    auth.logout_user();
    Ok(())
}

/// We can access the current user via `auth.current_user`.
/// We can have both anonymous user (id 1) and a logged in user (id 2).
///
/// Logged-in users will have more permissions which we can modify.
#[post("/api/user/name", auth: auth::Session)]
pub async fn get_user_name() -> Result<String> {
    Ok(auth.current_user.unwrap().username)
}

/// Get the current user's permissions, guarding the endpoint with the `Auth` validator.
/// If this returns false, we use the `or_unauthorized` extension to return a 401 error.
#[get("/api/user/permissions", auth: auth::Session)]
pub async fn get_permissions() -> Result<HashSet<String>> {
    use crate::auth::User;
    use axum_session_auth::{Auth, Rights};

    let user = auth.current_user.unwrap();

    Auth::<User, i64, sqlx::SqlitePool>::build([axum::http::Method::GET], false)
        .requires(Rights::any([
            Rights::permission("Category::View"),
            Rights::permission("Admin::View"),
        ]))
        .validate(&user, &axum::http::Method::GET, None)
        .await
        .or_unauthorized("You do not have permission to view categories")?;

    Ok(user.permissions)
}
