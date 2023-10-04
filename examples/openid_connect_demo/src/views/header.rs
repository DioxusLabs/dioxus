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
    let fermi_client = use_atom_ref(cx, &FERMI_CLIENT);
    let fermi_auth_token = use_atom_ref(cx, &FERMI_AUTH_TOKEN);
    cx.render(match fermi_client.read().oidc_client.clone() {
        Some(_client) => match fermi_auth_token.read().id_token.clone() {
            Some(id_token) => {
                let log_out_url_future =
                    use_future(cx, (), |_| async move { log_out_url(id_token).await });
                match log_out_url_future.value() {
                    Some(log_out_url) => {
                        rsx! {
                            Link {
                                onclick: move |_| {
                                    {
                                        AuthTokenState::use_persistent_set(fermi_auth_token, AuthTokenState::default());
                                    }
                                },
                                to: log_out_url.to_string(),
                                "Disconnect"
                            }
                        }
                    }
                    None => {
                        rsx! {{}}
                    }
                }
            }
            None => {
                rsx! {{}}
            }
        },
        None => {
            rsx! {{}}
        }
    })
}

#[component]
pub fn RefreshToken(cx: Scope) -> Element {
    let auth_token = use_atom_ref(cx, &FERMI_AUTH_TOKEN);
    let auth_request = use_atom_ref(cx, &FERMI_AUTH_REQUEST);
    let fermi_client = use_atom_ref(cx, &FERMI_CLIENT);
    cx.render(match fermi_client.read().oidc_client.clone() {
        Some(client) => match auth_token.read().refresh_token.clone() {
            Some(refresh_token) => {
                let exchange_refresh_token_future = use_future(cx, (), |_| async move {
                    exchange_refresh_token(client, refresh_token).await
                });
                match exchange_refresh_token_future.value() {
                    Some(response_token) => match response_token {
                        Ok(response_token) => {
                            AuthTokenState::use_persistent_set(
                                auth_token,
                                AuthTokenState {
                                    id_token: response_token.id_token().cloned(),
                                    refresh_token: response_token.refresh_token().cloned(),
                                },
                            );
                            rsx! { div { "Token refreshed successfully" } }
                        }
                        Err(error) => {
                            AuthTokenState::use_persistent_set(
                                auth_token,
                                AuthTokenState::default(),
                            );
                            AuthRequestState::use_persistent_set(
                                auth_request,
                                AuthRequestState::default(),
                            );
                            rsx! {div{"Error while trying to refresh the token: {error}"}}
                        }
                    },
                    None => {
                        rsx! { div { "Expired refresh token...Please wait" } }
                    }
                }
            }
            None => {
                rsx! { div { "Id token expired and no refresh token found" } }
            }
        },
        None => {
            rsx! {{}}
        }
    })
}

#[component]
pub fn AuthHeader(cx: Scope) -> Element {
    let auth_token = use_atom_ref(cx, &FERMI_AUTH_TOKEN);
    let fermi_auth_request = use_atom_ref(cx, &FERMI_AUTH_REQUEST);
    let fermi_client = use_atom_ref(cx, &FERMI_CLIENT);

    cx.render(match fermi_client.read().oidc_client.clone() {
        // Client successfully initialized
        Some(client) => match fermi_auth_request.read().clone().auth_request {
            // Nonce and authorization url already initialized
            Some(auth_request) => match &auth_token.read().id_token {
                Some(id_token) => {
                    match email(client.clone(), id_token.clone(), auth_request.nonce.clone()) {
                        Ok(email) => rsx! {
                            div {
                                div { email }
                                LogOut {}
                                Outlet::<Route> {}
                            }
                        },
                        // Id token failed to be decoded
                        Err(error) => match error {
                            //Id token failed to be decoded because it expired, we refresh it
                            openidconnect::ClaimsVerificationError::Expired(_message) => {
                                rsx! {
                                    div {
                                        RefreshToken {}
                                        Outlet::<Route> {}
                                    }
                                }
                            }
                            // Other issue with token decoding
                            _ => rsx! {
                                div {
                                    div { error.to_string() }
                                    Outlet::<Route> {}
                                }
                            },
                        },
                    }
                }
                // The user is probably not logged in
                None => {
                    rsx! {
                        div {
                            Link { to: auth_request.authorize_url.clone(), "Log in" }
                            Outlet::<Route> {}
                        }
                    }
                }
            },
            None => {
                let auth_request = authorize_url(client);
                AuthRequestState::use_persistent_set(
                    fermi_auth_request,
                    AuthRequestState {
                        auth_request: Some(auth_request),
                    },
                );
                rsx! { div { "Loading nonce" } }
            }
        },
        None => {
            let init_client_future =
                use_future(cx, (), |_| async move { init_oidc_client().await });
            if let Some(client) = init_client_future.value() {
                *fermi_client.write() = ClientState {
                    oidc_client: Some(client.clone()),
                };
            }
            rsx! {
                div {
                    div { "Loading client, please wait" }
                    Outlet::<Route> {}
                }
            }
        }
    })
}
