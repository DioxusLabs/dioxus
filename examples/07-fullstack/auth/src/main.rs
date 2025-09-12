#![allow(non_snake_case, unused)]

#[cfg(feature = "server")]
mod auth;

use dioxus::prelude::*;
use dioxus::Result;
use serde::{Deserialize, Serialize};

fn main() {
    #[cfg(feature = "server")]
    dioxus::fullstack::with_router(|| async {
        use crate::auth::*;
        use axum::routing::*;
        use axum_session::SessionConfig;
        use axum_session::SessionStore;
        use axum_session_auth::AuthConfig;
        use axum_session_sqlx::SessionSqlitePool;

        let pool = connect_to_database().await;

        //This Defaults as normal Cookies.
        //To enable Private cookies for integrity, and authenticity please check the next Example.
        let session_config = SessionConfig::default().with_table_name("test_table");
        let auth_config = AuthConfig::<i64>::default().with_anonymous_user_id(Some(1));
        let session_store =
            SessionStore::<SessionSqlitePool>::new(Some(pool.clone().into()), session_config)
                .await
                .unwrap();

        User::create_user_tables(&pool).await;

        // build our application with some routes
        let router = Router::new()
            .layer(
                axum_session_auth::AuthSessionLayer::<
                    crate::auth::User,
                    i64,
                    SessionSqlitePool,
                    sqlx::SqlitePool,
                >::new(Some(pool))
                .with_config(auth_config),
            )
            .layer(axum_session::SessionLayer::new(session_store));

        dioxus::Ok(router)
    });

    dioxus::launch(app);
}

fn app() -> Element {
    let mut user_name = use_signal(|| "?".to_string());
    let mut permissions = use_signal(|| "?".to_string());

    rsx! {
        div {
            button {
                onclick: move |_| async move {
                    login().await?;
                    Ok(())
                },
                "Login Test User"
            }
        }
        div {
            button {
                onclick: move |_| async move {
                    let data = get_user_name().await?;
                    user_name.set(data);
                    Ok(())
                },
                "Get User Name"
            }
            "User name: {user_name}"
        }
        div {
            button {
                onclick: move |_| async move {
                    let data = get_permissions().await?;
                    permissions.set(data);
                    Ok(())
                },
                "Get Permissions"
            }
            "Permissions: {permissions}"
        }
    }
}

#[get("/api/user/name")]
pub async fn get_user_name() -> Result<String> {
    let auth = auth::get_session().await?;
    Ok(auth.current_user.unwrap().username.to_string())
}

#[post("/api/user/login")]
pub async fn login() -> Result<()> {
    auth::get_session().await?.login_user(2);
    Ok(())
}

#[get("/api/user/permissions")]
pub async fn get_permissions() -> Result<String> {
    use axum_session_auth::{Auth, Rights};

    let method: axum::http::Method = extract().await?;
    let auth = auth::get_session().await?;
    let current_user = auth.current_user.clone().unwrap_or_default();

    // lets check permissions only and not worry about if they are anon or not
    if !Auth::<crate::auth::User, i64, sqlx::SqlitePool>::build([axum::http::Method::POST], false)
        .requires(Rights::any([
            Rights::permission("Category::View"),
            Rights::permission("Admin::View"),
        ]))
        .validate(&current_user, &method, None)
        .await
    {
        return Ok(format!(
            "User {}, Does not have permissions needed to view this page please login",
            current_user.username
        ));
    }

    Ok(format!(
        "User has Permissions needed. Here are the Users permissions: {:?}",
        current_user.permissions
    ))
}
