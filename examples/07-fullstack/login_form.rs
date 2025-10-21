//! This example demonstrates how to use types like `Form`, `SetHeader`, and `TypedHeader`
//! to create a simple login form that sets a cookie in the browser and uses it for authentication
//! on a protected endpoint.
//!
//! For more information on handling forms in general, see the multipart_form example.
//!
//! The intent with this example is to show how to use the building blocks like `Form` and `SetHeader`
//! to roll a simple authentication system.

use dioxus::fullstack::{Form, SetCookie, SetHeader};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use {
    dioxus::fullstack::{Cookie, TypedHeader},
    std::sync::LazyLock,
    uuid::Uuid,
};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut fetch_login = use_action(login);
    let mut fetch_sensitive = use_action(sensitive);

    rsx! {
        h1 { "Login Form Demo" }
        button {
            onclick: move |_| async move {
                fetch_sensitive.call();
            },
            "Get Sensitive Data",
        }
        pre { "Response from locked API: {fetch_sensitive.value():?}"}
        form {
            onsubmit: move |evt: FormEvent| async move {
                // Prevent the browser from navigating away.
                evt.prevent_default();

                // Extract the form values into our `LoginForm` struct. The `.parsed_values` method
                // is provided by Dioxus and works with any form element that has `name` attributes.
                let values: LoginForm = evt.parsed_values().unwrap();

                // Call our server function with the form values wrapped in `Form`. The `SetHeader`
                // response will set a cookie in the browser if the login is successful.
                fetch_login.call(Form(values)).await;

                // Now that we're logged in, we can call our sensitive endpoint.
                fetch_sensitive.call().await;
            },
            input { r#type: "text", id: "username", name: "username" }
            label { "Username" }
            input { r#type: "password", id: "password", name: "password" }
            label { "Password" }
            button { "Login" }
        }

    }
}

#[derive(Deserialize, Serialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

/// A static session ID for demonstration purposes. This forces all previous logins to be invalidated
/// when the server restarts.
#[cfg(feature = "server")]
static THIS_SESSION_ID: LazyLock<Uuid> = LazyLock::new(Uuid::new_v4);

/// In our `login` form, we'll return a `SetCookie` header if the login is successful.
///
/// This will set a cookie in the user's browser that can be used for subsequent authenticated requests.
/// The `SetHeader::new()` method takes anything that can be converted into a `HeaderValue`.
///
/// We can set multiple headers by returning a tuple of `SetHeader` types, or passing in a tuple
/// of headers to `SetHeader::new()`.
#[post("/api/login")]
async fn login(form: Form<LoginForm>) -> Result<SetHeader<SetCookie>> {
    // Verify the username and password. In a real application, you'd check these against a database.
    if form.0.username == "admin" && form.0.password == "password" {
        return Ok(SetHeader::new(format!("auth-demo={};", &*THIS_SESSION_ID))?);
    }

    HttpError::unauthorized("Invalid username or password")?
}

/// We'll use the `TypedHeader` extractor on the server to get the cookie from the request.
#[get("/api/sensitive", header: TypedHeader<Cookie>)]
async fn sensitive() -> Result<String> {
    // Extract the cookie from the request headers and use `.eq` to verify its value.
    // The `or_unauthorized` works on boolean values, returning a 401 if the condition is false.
    header
        .get("auth-demo")
        .or_unauthorized("Missing auth-demo cookie")?
        .eq(THIS_SESSION_ID.to_string().as_str())
        .or_unauthorized("Invalid auth-demo cookie")?;

    Ok("Sensitive data".to_string())
}
