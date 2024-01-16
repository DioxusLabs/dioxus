use crate::{
    oidc::{
        authorize_url, email, exchange_refresh_token, init_oidc_client, log_out_url,
        AuthRequestState, AuthTokenState, ClientState,
    },
    props::client::ClientProps,
    router::Route,
    storage::PersistentWrite,
    FERMI_AUTH_REQUEST, FERMI_AUTH_TOKEN, FERMI_CLIENT,
};
use dioxus::prelude::*;
use dioxus_router::prelude::{Link, Outlet};
use fermi::*;
use openidconnect::{url::Url, OAuth2TokenResponse, TokenResponse};

#[component]
pub fn LogOut(cx: Scope<ClientProps>) -> Element {
    let fermi_auth_token = use_atom_ref(&FERMI_AUTH_TOKEN);
    let fermi_auth_token_read = fermi_auth_token.read().clone();
    let log_out_url_state = use_signal(|| None::<Option<Result<Url, crate::errors::Error>>>);
    cx.render(match fermi_auth_token_read {
        Some(fermi_auth_token_read) => match fermi_auth_token_read.id_token.clone() {
            Some(id_token) => match log_out_url_state.get() {
                Some(log_out_url_result) => match log_out_url_result {
                    Some(uri) => match uri {
                        Ok(uri) => {
                            render! {
                                Link {
                                    onclick: move |_| {
                                        {
                                            AuthTokenState::persistent_set(
                                                fermi_auth_token,
                                                Some(AuthTokenState::default()),
                                            );
                                        }
                                    },
                                    to: uri.to_string(),
                                    "Log out"
                                }
                            }
                        }
                        Err(error) => {
                            render! { div { "Failed to load disconnection url: {error:?}" } }
                        }
                    },
                    None => {
                        render! { div { "Loading... Please wait" } }
                    }
                },
                None => {
                    let logout_url_task = move || {
                        cx.spawn({
                            let log_out_url_state = log_out_url_state.to_owned();
                            async move {
                                let logout_url = log_out_url(id_token).await;
                                let logout_url_option = Some(logout_url);
                                log_out_url_state.set(Some(logout_url_option));
                            }
                        })
                    };
                    logout_url_task();
                    render! { div { "Loading log out url... Please wait" } }
                }
            },
            None => {
                render! {{}}
            }
        },
        None => {
            render! {{}}
        }
    })
}

#[component]
pub fn RefreshToken(cx: Scope<ClientProps>) -> Element {
    let fermi_auth_token = use_atom_ref(&FERMI_AUTH_TOKEN);
    let fermi_auth_request = use_atom_ref(&FERMI_AUTH_REQUEST);
    let fermi_auth_token_read = fermi_auth_token.read().clone();
    cx.render(match fermi_auth_token_read {
        Some(fermi_auth_client_read) => match fermi_auth_client_read.refresh_token {
            Some(refresh_token) => {
                let fermi_auth_token = fermi_auth_token.to_owned();
                let fermi_auth_request = fermi_auth_request.to_owned();
                let client = cx.props.client.clone();
                let exchange_refresh_token_spawn = move || {
                    cx.spawn({
                        async move {
                            let exchange_refresh_token =
                                exchange_refresh_token(client, refresh_token).await;
                            match exchange_refresh_token {
                                Ok(response_token) => {
                                    AuthTokenState::persistent_set(
                                        &fermi_auth_token,
                                        Some(AuthTokenState {
                                            id_token: response_token.id_token().cloned(),
                                            refresh_token: response_token.refresh_token().cloned(),
                                        }),
                                    );
                                }
                                Err(_error) => {
                                    AuthTokenState::persistent_set(
                                        &fermi_auth_token,
                                        Some(AuthTokenState::default()),
                                    );
                                    AuthRequestState::persistent_set(
                                        &fermi_auth_request,
                                        Some(AuthRequestState::default()),
                                    );
                                }
                            }
                        }
                    })
                };
                exchange_refresh_token_spawn();
                render! { div { "Refreshing session, please wait" } }
            }
            None => {
                render! { div { "Id token expired and no refresh token found" } }
            }
        },
        None => {
            render! {{}}
        }
    })
}

#[component]
pub fn LoadClient() -> Element {
    let init_client_future = use_future(|| async move { init_oidc_client().await });
    let fermi_client: &UseAtomRef<ClientState> = use_atom_ref(&FERMI_CLIENT);
    cx.render(match init_client_future.value() {
        Some(client_props) => match client_props {
            Ok((client_id, client)) => {
                *fermi_client.write() = ClientState {
                    oidc_client: Some(ClientProps::new(client_id.clone(), client.clone())),
                };
                render! {
                    div { "Client successfully loaded" }
                    Outlet::<Route> {}
                }
            }
            Err(error) => {
                log::info! {"Failed to load client: {:?}", error};
                render! {
                    div { "Failed to load client: {error:?}" }
                    Outlet::<Route> {}
                }
            }
        },
        None => {
            render! {
                div {
                    div { "Loading client, please wait" }
                    Outlet::<Route> {}
                }
            }
        }
    })
}

#[component]
pub fn AuthHeader() -> Element {
    let auth_token = use_atom_ref(&FERMI_AUTH_TOKEN);
    let fermi_auth_request = use_atom_ref(&FERMI_AUTH_REQUEST);
    let fermi_client: &UseAtomRef<ClientState> = use_atom_ref(&FERMI_CLIENT);
    let client = fermi_client.read().oidc_client.clone();
    let auth_request_read = fermi_auth_request.read().clone();
    let auth_token_read = auth_token.read().clone();
    cx.render(match (client, auth_request_read, auth_token_read) {
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
                                    render! {
                                        div {
                                            div { {email} }
                                            LogOut { client_id: client_props.client_id, client: client_props.client }
                                            Outlet::<Route> {}
                                        }
                                    }
                                }
                                // Id token failed to be decoded
                                Err(error) => match error {
                                    // Id token failed to be decoded because it expired, we refresh it
                                    openidconnect::ClaimsVerificationError::Expired(_message) => {
                                        log::info!("Token expired");
                                        render! {
                                            div {
                                                RefreshToken { client_id: client_props.client_id, client: client_props.client }
                                                Outlet::<Route> {}
                                            }
                                        }
                                    }
                                    // Other issue with token decoding
                                    _ => {
                                        log::info!("Other issue with token");
                                        render! {
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
                            render! {
                                div {
                                    Link { to: auth_request.authorize_url.clone(), "Log in" }
                                    Outlet::<Route> {}
                                }
                            }
                        }
                    }
                }
                None => {
                    let auth_request = authorize_url(client_props.client);
                    AuthRequestState::persistent_set(
                        fermi_auth_request,
                        Some(AuthRequestState {
                            auth_request: Some(auth_request),
                        }),
                    );
                    render! { div { "Loading nonce" } }
                }
            }
        }
        // Client is not initialized yet, we need it for everything
        (None, _, _) => {
            render! { LoadClient {} }
        }
        // We need everything loaded before doing anything
        (_client, _auth_request, _auth_token) => {
            render! {{}}
        }
    })
}
