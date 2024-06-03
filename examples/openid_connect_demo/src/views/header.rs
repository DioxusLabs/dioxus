use crate::{
    oidc::{
        authorize_url, email, exchange_refresh_token, init_oidc_client, log_out_url,
        AuthRequestState, AuthTokenState, ClientState,
    },
    props::client::ClientProps,
    router::Route,
    storage::PersistentWrite,
    AUTH_REQUEST, AUTH_TOKEN, CLIENT,
};
use anyhow::Result;
use dioxus::prelude::*;
use dioxus::router::prelude::{Link, Outlet};
use openidconnect::{url::Url, OAuth2TokenResponse, TokenResponse};

#[component]
pub fn LogOut() -> Element {
    let AUTH_TOKEN_read = AUTH_TOKEN.read().clone();
    let log_out_url_state = use_signal(|| None::<Option<Result<Url>>>);
    match AUTH_TOKEN_read {
        Some(AUTH_TOKEN_read) => match AUTH_TOKEN_read.id_token.clone() {
            Some(id_token) => match &*log_out_url_state.read() {
                Some(log_out_url_result) => match log_out_url_result {
                    Some(uri) => match uri {
                        Ok(uri) => {
                            rsx! {
                                Link {
                                    onclick: move |_| {
                                        {
                                            AuthTokenState::persistent_set(
                                                AuthTokenState::default()
                                            );
                                        }
                                    },
                                    to: uri.to_string(),
                                    "Log out"
                                }
                            }
                        }
                        Err(error) => {
                            rsx! { div { "Failed to load disconnection url: {error:?}" } }
                        }
                    },
                    None => {
                        rsx! { div { "Loading... Please wait" } }
                    }
                },
                None => {
                    let logout_url_task = move || {
                        spawn({
                            let mut log_out_url_state = log_out_url_state.to_owned();
                            async move {
                                let logout_url = log_out_url(id_token).await;
                                let logout_url_option = Some(logout_url);
                                log_out_url_state.set(Some(logout_url_option));
                            }
                        })
                    };
                    logout_url_task();
                    rsx! { div { "Loading log out url... Please wait" } }
                }
            },
            None => {
                rsx! {{}}
            }
        },
        None => {
            rsx! {{}}
        }
    }
}

#[component]
pub fn RefreshToken(props: ClientProps) -> Element {
    let auth_token = AUTH_TOKEN.read().clone();
    match auth_token {
        Some(auth_token) => match auth_token.refresh_token {
            Some(refresh_token) => {
                rsx! { div {
                    onmounted: {
                        move |_| {
                            let client = props.client.clone();
                            let refresh_token = refresh_token.clone();
                            async move {
                                let exchange_refresh_token =
                                    exchange_refresh_token(client, refresh_token).await;
                                match exchange_refresh_token {
                                    Ok(response_token) => {
                                        AuthTokenState::persistent_set(AuthTokenState {
                                            id_token: response_token.id_token().cloned(),
                                            refresh_token: response_token.refresh_token().cloned(),
                                        });
                                    }
                                    Err(_error) => {
                                        AuthTokenState::persistent_set(AuthTokenState::default());
                                        AuthRequestState::persistent_set(AuthRequestState::default());
                                    }
                                }
                            }
                        }
                    },
                    "Refreshing session, please wait"
                } }
            }
            None => {
                rsx! { div { "Id token expired and no refresh token found" } }
            }
        },
        None => {
            rsx! {{}}
        }
    }
}

#[component]
pub fn LoadClient() -> Element {
    let init_client_future = use_resource(move || async move { init_oidc_client().await });
    match &*init_client_future.value().read() {
        Some(client_props) => match client_props {
            Ok((client_id, client)) => {
                rsx! {
                    div {
                        onmounted: {
                            let client_id = client_id.clone();
                            let client = client.clone();
                            move |_| {
                                *CLIENT.write() = ClientState {
                                    oidc_client: Some(ClientProps::new(client_id.clone(), client.clone())),
                                };
                            }
                        },
                        "Client successfully loaded"
                    }
                    Outlet::<Route> {}
                }
            }
            Err(error) => {
                log::info! {"Failed to load client: {:?}", error};
                rsx! {
                    div { "Failed to load client: {error:?}" }
                    Outlet::<Route> {}
                }
            }
        },
        None => {
            rsx! {
                div {
                    div { "Loading client, please wait" }
                    Outlet::<Route> {}
                }
            }
        }
    }
}

#[component]
pub fn AuthHeader() -> Element {
    let client = CLIENT.read().oidc_client.clone();
    let auth_request = AUTH_REQUEST.read().clone();
    let auth_token = AUTH_TOKEN.read().clone();
    match (client, auth_request, auth_token) {
        // We have everything we need to attempt to authenticate the user
        (Some(client_props), Some(auth_request), Some(auth_token)) => {
            match auth_request.auth_request {
                Some(auth_request) => {
                    match auth_token.id_token {
                        Some(id_token) => {
                            match email(
                                client_props.client.clone(),
                                id_token.clone(),
                                auth_request.nonce.clone(),
                            ) {
                                Ok(email) => {
                                    rsx! {
                                        div {
                                            div { {email} }
                                            LogOut {}
                                            Outlet::<Route> {}
                                        }
                                    }
                                }
                                // Id token failed to be decoded
                                Err(error) => match error {
                                    // Id token failed to be decoded because it expired, we refresh it
                                    openidconnect::ClaimsVerificationError::Expired(_message) => {
                                        log::info!("Token expired");
                                        rsx! {
                                            div {
                                                RefreshToken { client_id: client_props.client_id, client: client_props.client }
                                                Outlet::<Route> {}
                                            }
                                        }
                                    }
                                    // Other issue with token decoding
                                    _ => {
                                        log::info!("Other issue with token");
                                        rsx! {
                                            div {
                                                div { "{error}" }
                                                Outlet::<Route> {}
                                            }
                                        }
                                    }
                                },
                            }
                        }
                        // User is not logged in
                        None => {
                            rsx! {
                                div {
                                    Link { to: auth_request.authorize_url.clone(), "Log in" }
                                    Outlet::<Route> {}
                                }
                            }
                        }
                    }
                }
                None => {
                    rsx! { div {
                        onmounted: {
                            let client = client_props.client;
                            move |_| {
                                let auth_request = authorize_url(client.clone());
                                AuthRequestState::persistent_set(AuthRequestState {
                                    auth_request: Some(auth_request),
                                });
                            }
                        },
                        "Loading nonce"
                    } }
                }
            }
        }
        // Client is not initialized yet, we need it for everything
        (None, _, _) => {
            rsx! { LoadClient {} }
        }
        // We need everything loaded before doing anything
        (_client, _auth_request, _auth_token) => {
            rsx! {{}}
        }
    }
}
