use dioxus::prelude::*;

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
    dioxus::serve(|| async {
        use crate::auth::*;
        use axum::routing::*;
        use axum_session::{SessionConfig, SessionLayer, SessionStore};
        use axum_session_auth::AuthConfig;
        use axum_session_sqlx::SessionSqlitePool;
        use sqlx::{sqlite::SqlitePoolOptions, Executor};

        // Create an in-memory SQLite database and set up our tables
        let db = SqlitePoolOptions::new()
            .max_connections(5)
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
        Ok(Router::new()
            .serve_dioxus_application(ServeConfig::new().unwrap(), app)
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

fn app() -> Element {
    let mut login = use_action(|_| login());
    let mut user_name = use_action(|_| get_user_name());
    let mut permissions = use_action(|_| get_permissions());
    let mut logout = use_action(|_| logout());

    rsx! {
        button { onclick: move |_| login.dispatch(()), "Login Test User" }
        button { onclick: move |_| user_name.dispatch(()), "Get User Name" }
        button { onclick: move |_| permissions.dispatch(()), "Get Permissions" }
        button {
            onclick: move |_| async move {
                logout.dispatch(()).await;
                login.reset();
                user_name.reset();
                permissions.reset();
            },
            "Reset"
        }
        pre { "Logged in: {login.result():?}" }
        pre { "User name: {user_name.result():?}" }
        pre { "Permissions: {permissions.result():?}" }
    }
}

#[post("/api/user/logout", auth: auth::Session)]
pub async fn logout() -> Result<()> {
    auth.logout_user();
    Ok(())
}

#[post("/api/user/login", auth: auth::Session)]
pub async fn login() -> Result<()> {
    auth.login_user(2);
    Ok(())
}

#[get("/api/user/name", auth: auth::Session)]
pub async fn get_user_name() -> Result<String> {
    Ok(auth.current_user.unwrap().username)
}

#[get("/api/user/permissions", auth: auth::Session)]
pub async fn get_permissions() -> Result<String> {
    use crate::auth::User;
    use axum_session_auth::{Auth, Rights};
    let user = auth.current_user.unwrap();

    // lets check permissions only and not worry about if they are anon or not
    if !Auth::<User, i64, sqlx::SqlitePool>::build([axum::http::Method::GET], false)
        .requires(Rights::any([
            Rights::permission("Category::View"),
            Rights::permission("Admin::View"),
        ]))
        .validate(&user, &axum::http::Method::GET, None)
        .await
    {
        return Ok(format!(
            "User does not have Permissions needed. - {:?} ",
            user.permissions
        ));
    }

    Ok(format!(
        "User has Permissions needed. {:?}",
        user.permissions
    ))
}
