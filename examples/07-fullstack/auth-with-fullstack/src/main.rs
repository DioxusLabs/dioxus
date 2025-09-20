use axum::extract::{FromRequest, FromRequestParts};
use dioxus::{
    fullstack::{DioxusServerState, FromResponse, HttpError, ServerFnEncoder},
    prelude::*,
};
use http::HeaderMap;

#[cfg(feature = "server")]
mod auth;

fn main() {
    // On the client, we simply launch the app as normal, taking over the main thread
    #[cfg(not(feature = "server"))]
    dioxus::launch(app);

    // On the server, we can use `dioxus::serve` to create a server that serves our app.
    // The `serve` function takes a `Result<T>` where `T` is a tower service (and thus an axum router).
    // The `app` parameter is mounted as a fallback service to handle HTML rendering and static assets.
    #[cfg(feature = "server")]
    dioxus::serve(app, || async {
        use crate::auth::*;
        use axum::routing::*;
        use axum_session::{SessionConfig, SessionLayer, SessionStore};
        use axum_session_auth::{AuthConfig, AuthSessionLayer};
        use axum_session_sqlx::SessionSqlitePool;
        use sqlx::{sqlite::SqlitePoolOptions, Executor, SqlitePool};

        // Create an in-memory SQLite database and setup our tables
        let db = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with("sqlite::memory:".parse()?)
            .await?;

        // Create the users
        db.execute(r#"CREATE TABLE IF NOT EXISTS users ( "id" INTEGER PRIMARY KEY, "anonymous" BOOLEAN NOT NULL, "username" VARCHAR(256) NOT NULL )"#,)
            .await?;
        db.execute(r#"CREATE TABLE IF NOT EXISTS user_permissions ( "user_id" INTEGER NOT NULL, "token" VARCHAR(256) NOT NULL)"#,)
            .await?;
        db.execute(r#"INSERT INTO users (id, anonymous, username) SELECT 1, true, 'Guest' ON CONFLICT(id) DO UPDATE SET anonymous = EXCLUDED.anonymous, username = EXCLUDED.username"#,)
            .await?;
        db.execute(r#"INSERT INTO users (id, anonymous, username) SELECT 2, false, 'Test' ON CONFLICT(id) DO UPDATE SET anonymous = EXCLUDED.anonymous, username = EXCLUDED.username"#,)
            .await?;
        db.execute(r#"INSERT INTO user_permissions (user_id, token) SELECT 2, 'Category::View'"#)
            .await?;

        // This Defaults as normal Cookies.
        // build our application with some routes
        Ok(Router::new()
            .layer(
                AuthSessionLayer::<User, i64, SessionSqlitePool, SqlitePool>::new(Some(db.clone()))
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

fn app() -> Element {
    let mut user_name = use_action(|_| get_user_name());
    let mut permissions = use_action(|_| get_permissions());
    let mut login = use_action(|_| login());

    rsx! {
        button { onclick: move |_| login.dispatch(()), "Login Test User" }
        button { onclick: move |_| user_name.dispatch(()), "Get User Name" }
        button { onclick: move |_| permissions.dispatch(()), "Get Permissions" }
        pre { "User name: {user_name:?}" }
        pre { "Permissions: {permissions:?}" }
    }
}

async fn get_user_name() -> Result<()> {
    todo!()
}
async fn get_permissions() -> Result<()> {
    todo!()
}
async fn login() -> Result<()> {
    todo!()
}

// #[get("/api/user/name", auth: auth::Session)]
// pub async fn get_user_name() -> Result<String> {
//     Ok(auth.current_user.or_unauthorized("")?.username)
// }

// #[post("/api/user/login", auth: auth::Session)]
// pub async fn login() -> Result<()> {
//     auth.login_user(2);
//     Ok(())
// }

// #[get("/api/user/permissions", auth: auth::Session)]
// pub async fn get_permissions() -> Result<String> {
//     use crate::auth::User;
//     use axum_session_auth::{Auth, Rights};

//     let user = auth.current_user.or_unauthorized("No current user")?;

//     // lets check permissions only and not worry about if they are anon or not
//     Auth::<User, i64, sqlx::SqlitePool>::build([axum::http::Method::POST], false)
//         .requires(Rights::any([
//             Rights::permission("Category::View"),
//             Rights::permission("Admin::View"),
//         ]))
//         .validate(&user, &axum::http::Method::GET, None)
//         .await
//         .or_unauthorized("You do not have permissions to view this page")?;

//     Ok(format!(
//         "User has Permissions needed. {:?}",
//         user.permissions
//     ))
// }

// #[get("/api/user/permissions")]
pub async fn do_thing(auth: auth::Session) -> Result<()> {
    struct ___Body_Serialize___<#[cfg(feature = "server")] A> {
        #[cfg(feature = "server")]
        auth: A,
    }
    use dioxus::fullstack::ExtractRequest;
    let __state = DioxusServerState::default();

    let request = axum::extract::Request::default();

    let (auth,) =
        (&&&&&&&&&ServerFnEncoder::<___Body_Serialize___<auth::Session>, (auth::Session,)>::new())
            .extract_axum(__state, request, |data| (data.auth,))
            .await
            .unwrap();

    todo!()
}
