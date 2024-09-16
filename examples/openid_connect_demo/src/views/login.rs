use crate::{
    oidc::{token_response, AuthTokenState},
    router::Route,
    storage::{auth_request, use_auth_token},
    CLIENT,
};
use dioxus::prelude::*;
use dioxus::router::prelude::Link;
use openidconnect::{OAuth2TokenResponse, TokenResponse};

#[component]
pub fn Login(query_string: String) -> Element {
    let client = CLIENT.read().oidc_client.clone();
    let mut auth_token = use_auth_token();
    let current_auth_token = auth_token();
    match client {
        Some(client_props) => {
            match (
                current_auth_token.id_token,
                current_auth_token.refresh_token,
            ) {
                (Some(_id_token), Some(_refresh_token)) => {
                    rsx! {
                        div { "Sign in successful" }
                        Link { to: Route::Home {}, "Go back home" }
                    }
                }
                // If the refresh token is set but not the id_token, there was an error, we just go back home and reset their value
                (None, Some(_)) | (Some(_), None) => {
                    rsx! {
                        div { "Error while attempting to log in" }
                        Link {
                            to: Route::Home {},
                            onclick: move |_| {
                                auth_token.take();
                                auth_request().take();
                            },
                            "Go back home"
                        }
                    }
                }
                (None, None) => {
                    let mut query_pairs = form_urlencoded::parse(query_string.as_bytes());
                    let code_pair = query_pairs.find(|(key, _value)| key == "code");
                    match code_pair {
                        Some((_key, code)) => {
                            let code = code.to_string();
                            rsx! { div {
                                onmounted: {
                                    move |_| {
                                        let auth_code = code.to_string();
                                        let client_props = client_props.clone();
                                        async move {
                                            let token_response_result =
                                                token_response(client_props.client, auth_code).await;
                                            match token_response_result {
                                                Ok(token_response) => {
                                                    let id_token = token_response.id_token().unwrap();
                                                    auth_token.set(AuthTokenState {
                                                        id_token: Some(id_token.clone()),
                                                        refresh_token: token_response
                                                            .refresh_token()
                                                            .cloned(),
                                                    });
                                                }
                                                Err(error) => {
                                                    log::warn! {"{error}"};
                                                }
                                            }
                                        }
                                    }
                                }
                            }}
                        }
                        None => {
                            rsx! { div { "No code provided" } }
                        }
                    }
                }
            }
        }
        _ => {
            rsx! {{}}
        }
    }
}
