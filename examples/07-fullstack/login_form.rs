//! Implementing a login form

use dioxus::fullstack::{Cookie, Form, SetCookie, SetHeader};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        h1 { "Login Form Demo" }
        button {
            onclick: move |_| async move {
                let result = sensitive().await;
                info!("Sensitive data: {:?}", result);
            },
            "Get Sensitive Data",
        }
        form {
            onsubmit: move |evt: FormEvent| {
                evt.prevent_default();
                async move {
                    info!("Form submitted: {:?}", evt.values());
                    let values: LoginForm = evt.parsed_values().unwrap();

                    let result = login(Form(values)).await;
                    info!("Login result: {:?}", result);
                    Ok(())
                }
            },
            input { r#type: "text", id: "username", name: "username" }
            label { "Username" }
            input { r#type: "password", id: "password", name: "password" }
            label { "Password" }
            button { "Login" }
        }

    }
}

#[cfg(feature = "server")]
type MyTypedHeader = dioxus::fullstack::TypedHeader<Cookie>;

#[derive(Deserialize, Serialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

/// In our `login` form, we'll return a `SetCookie` header if the login is successful.
///
/// This will set a cookie in the user's browser that can be used for subsequent authenticated requests.
/// The `SetHeader::new()` method takes anything that can be converted into a `HeaderValue`.
///
/// We can set multiple headers by returning a tuple of `SetHeader` types, or passing in a tuple
/// of headers to `SetHeader::new()`.
#[post("/api/login")]
async fn login(form: Form<LoginForm>) -> Result<SetHeader<SetCookie>> {
    if form.0.username == "admin" && form.0.password == "password" {
        return Ok(SetHeader::new("auth-demo=abcdef123456;")?);
    }

    HttpError::unauthorized("Invalid username or password")?
}

/// We'll use the `TypedHeader` extractor to get the cookie from the request.
#[get("/api/sensitive", header: MyTypedHeader)]
async fn sensitive() -> Result<String> {
    // Extract the cookie from the request headers and use `.eq` to verify its value.
    // The `or_unauthorized` works on boolean values, returning a 401 if the condition is false.
    header
        .get("auth-demo")
        .or_unauthorized("Missing auth-demo cookie")?
        .eq("abcdef123456")
        .or_unauthorized("Invalid auth-demo cookie")?;

    Ok("Sensitive data".to_string())
}
