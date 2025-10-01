#[cfg(feature = "server")]
mod auth;

use dioxus::prelude::*;

fn main() {
    #[cfg(feature = "server")]
    dioxus::serve(|| async {
        use crate::auth::*;
        use axum_session::{SessionConfig, SessionLayer, SessionStore};
        use axum_session_auth::AuthConfig;
        use axum_session_sqlx::SessionSqlitePool;
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use std::str::FromStr;

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(SqliteConnectOptions::from_str("sqlite::memory:").unwrap())
            .await?;

        // This Defaults as normal Cookies.
        // To enable Private cookies for integrity, and authenticity please check the next Example.
        let session_config = SessionConfig::default().with_table_name("test_table");
        let auth_config = AuthConfig::<i64>::default().with_anonymous_user_id(Some(1));
        let session_store =
            SessionStore::<SessionSqlitePool>::new(Some(pool.clone().into()), session_config)
                .await
                .unwrap();

        User::create_user_tables(&pool).await?;

        // build our application with some routes
        Ok(
            dioxus::server::router(app)
                .layer(
                    axum_session_auth::AuthSessionLayer::<
                        User,
                        i64,
                        SessionSqlitePool,
                        sqlx::SqlitePool,
                    >::new(Some(pool))
                    .with_config(auth_config),
                )
                .layer(SessionLayer::new(session_store)),
        )
    });

    dioxus::launch(app);
}

fn app() -> Element {
    let mut user_name = use_action(get_user_name);
    let mut permissions = use_action(get_permissions);
    let mut login = use_action(login);

    rsx! {
        div {
            button {
                onclick: move |_|  {
                    login.call();
                },
                "Login Test User"
            }
        }
        div {
            button {
                onclick: move |_| {
                    user_name.call();
                },
                "Get User Name"
            }
            "User name: {user_name.value().unwrap_or_default()}"
        }
        div {
            button {
                onclick: move |_| {
                    permissions.call();
                },
                "Get Permissions"
            }
            "Permissions: {permissions.value().unwrap_or_default()}"
        }
    }
}

#[get("/api/user/name", auth: auth::Session)]
pub async fn get_user_name() -> Result<String> {
    Ok(auth
        .current_user
        .context("UNAUTHORIZED")?
        .username
        .to_string())
}

#[post("/api/user/login", auth: auth::Session)]
pub async fn login() -> Result<()> {
    auth.login_user(2);
    Ok(())
}

#[get("/api/user/permissions", auth: auth::Session)]
pub async fn get_permissions() -> Result<String> {
    use crate::auth::User;
    use axum_session_auth::{Auth, Rights};

    let user = auth.current_user.as_ref().context("UNAUTHORIZED")?;

    // lets check permissions only and not worry about if they are anon or not
    let has_permissions =
        Auth::<User, i64, sqlx::SqlitePool>::build([axum::http::Method::POST], false)
            .requires(Rights::any([
                Rights::permission("Category::View"),
                Rights::permission("Admin::View"),
            ]))
            .validate(user, &axum::http::Method::GET, None)
            .await;

    if !has_permissions {
        dioxus::core::bail!(
            "User {}, Does not have permissions needed to view this page please login",
            user.username
        );
    }

    Ok(format!(
        "User has Permissions needed. {:?}",
        user.permissions
    ))
}
