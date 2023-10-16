use crate::{
    oidc::{
        authorize_url, email, exchange_refresh_token, init_oidc_client, log_out_url,
        AuthRequestState, AuthTokenState, ClientState,
    },
    router::Route,
    storage::PersistentWrite,
    FERMI_AUTH_REQUEST, FERMI_AUTH_TOKEN, FERMI_CLIENT,
};
use dioxus::prelude::*;
use dioxus_router::prelude::{Link, Outlet};
use fermi::*;
use openidconnect::{OAuth2TokenResponse, TokenResponse};

#[component]
pub fn LogOut(cx: Scope) -> Element {
    let fermi_client: &UseAtomRef<ClientState> = use_atom_ref(cx, &FERMI_CLIENT);
    let fermi_auth_token = use_atom_ref(cx, &FERMI_AUTH_TOKEN);
    let client = fermi_client.read().oidc_client.clone();
    let fermi_auth_token_read = fermi_auth_token.read().clone();
    let log_out_url_state = use_state(cx, || None);
    cx.render(match (client, fermi_auth_token_read) {
        (Some(_client), Some(fermi_auth_token_read)) => {
            match fermi_auth_token_read.id_token.clone() {
                Some(id_token) => {
                    let logout_url_task = move || {
                        cx.spawn({
                            let log_out_url_state = log_out_url_state.to_owned();
                            async move {
                                let logout_url = log_out_url(id_token).await;
                                log_out_url_state.set(Some(logout_url));
                            }
                        })
                    };
                    logout_url_task();
                    match log_out_url_state.get() {
                        Some(uri) => match uri {
                            Ok(uri) => {
                                rsx! {
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
                                rsx! {
                                    div{format!{"Failed to load disconnection url: {:?}", error}}
                                }
                            }
                        },
                        None => {
                            rsx! { div { "Loading... Please wait" } }
                        }
                    }
                }
                None => {
                    rsx! {{}}
                }
            }
        }
        (_client, _fermi_auth_token) => {
            rsx! {{}}
        }
    })
}

#[component]
pub fn RefreshToken(cx: Scope) -> Element {
    let fermi_auth_token = use_atom_ref(cx, &FERMI_AUTH_TOKEN);
    let fermi_auth_request = use_atom_ref(cx, &FERMI_AUTH_REQUEST);
    let fermi_client = use_atom_ref(cx, &FERMI_CLIENT);
    let client = fermi_client.read().oidc_client.clone();
    let fermi_auth_token_read = fermi_auth_token.read().clone();
    cx.render(match (client, fermi_auth_token_read) {
        (Some(client), Some(fermi_auth_client_read)) => {
            match fermi_auth_client_read.refresh_token {
                Some(refresh_token) => {
                    let fermi_auth_token = fermi_auth_token.to_owned();
                    let fermi_auth_request = fermi_auth_request.to_owned();
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
                                                refresh_token: response_token
                                                    .refresh_token()
                                                    .cloned(),
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
                    rsx! { div { "Refreshing session, please wait" } }
                }
                None => {
                    rsx! { div { "Id token expired and no refresh token found" } }
                }
            }
        }
        // Either the client or the auth_client is None
        (client, fermi_auth_client_read) => {
            log::info!("{:?} {:?}", client, fermi_auth_client_read);
            rsx! {{}}
        }
    })
}

#[component]
pub fn LoadClient(cx: Scope) -> Element {
    let init_client_future = use_future(cx, (), |_| async move { init_oidc_client().await });
    let fermi_client: &UseAtomRef<ClientState> = use_atom_ref(cx, &FERMI_CLIENT);
    cx.render(match init_client_future.value() {
        Some(client) => match client {
            Ok(client) => {
                *fermi_client.write() = ClientState {
                    oidc_client: Some(client.clone()),
                };
                rsx! {
                    div{"Client successfully loaded"}
                    Outlet::<Route>{}
                }
            }
            Err(error) => {
                rsx! {
                    div{format!{"Failed to load client: {:?}", error}}
                    log::info!{"Failed to load client: {:?}", error}
                    Outlet::<Route>{}
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
    })
}

#[component]
pub fn AuthHeader(cx: Scope) -> Element {
    let auth_token = use_atom_ref(cx, &FERMI_AUTH_TOKEN);
    let fermi_auth_request = use_atom_ref(cx, &FERMI_AUTH_REQUEST);
    let fermi_client: &UseAtomRef<ClientState> = use_atom_ref(cx, &FERMI_CLIENT);
    let client = fermi_client.read().oidc_client.clone();
    let auth_request_read = fermi_auth_request.read().clone();
    let auth_token_read = auth_token.read().clone();
    cx.render(match (client, auth_request_read, auth_token_read) {
        // We have everything we need to attempt to authenticate the user
        (Some(client), Some(auth_request), Some(auth_token)) => {
            match auth_request.auth_request {
                Some(auth_request) => {
                    match auth_token.id_token {
                        Some(id_token) => {
                            match email(
                                client.clone(),
                                id_token.clone(),
                                auth_request.nonce.clone(),
                            ) {
                                Ok(email) => {
                                    rsx! {
                                        div {
                                            div { email }
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
                                                RefreshToken {}
                                                Outlet::<Route> {}
                                            }
                                        }
                                    }
                                    // Other issue with token decoding
                                    _ => {
                                        log::info!("Other issue with token");
                                        rsx! {
                                            div {
                                                div { error.to_string() }
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
                    let auth_request = authorize_url(client);
                    AuthRequestState::persistent_set(
                        fermi_auth_request,
                        Some(AuthRequestState {
                            auth_request: Some(auth_request),
                        }),
                    );
                    rsx! { div { "Loading nonce" } }
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
    })
}
