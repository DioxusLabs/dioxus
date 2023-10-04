use crate::{
    env::DIOXUS_FRONT_URL,
    oidc::{ token_response, AuthRequestState, AuthTokenState,
    },
    router::Route, FERMI_CLIENT, FERMI_AUTH_REQUEST, FERMI_AUTH_TOKEN,
    storage::PersistentWrite
};
use dioxus::prelude::*;
use dioxus_router::prelude::{Link, NavigationTarget};
use fermi::*;
use openidconnect::{OAuth2TokenResponse, TokenResponse};

#[component]
pub fn Login(cx: Scope, query_string: String) -> Element {
    let fermi_client = use_atom_ref(cx, &FERMI_CLIENT);
    let fermi_auth_token = use_atom_ref(cx, &FERMI_AUTH_TOKEN);
    let home_url: NavigationTarget<Route> = DIOXUS_FRONT_URL.parse().unwrap();
    let fermi_auth_request = use_atom_ref(cx, &FERMI_AUTH_REQUEST);

    cx.render(match fermi_client.read().oidc_client.clone() {
        Some(client) => 
        match fermi_auth_token.read().id_token.clone() {
            Some(_id_token) => match fermi_auth_token.read().refresh_token.clone() {
                Some(_refresh_token) => {
                    rsx! {
                        div{"Sign in successful"}
                        Link{
                            to: home_url,"Go back home"
                        }
                    }
                }
                None => {
                    rsx! {
                        div{"Error while attempting to log in"}
                        Link{
                            to: home_url, "Go back home", onclick: move |_|{
                                AuthTokenState::use_persistent_set(fermi_auth_token, AuthTokenState::default());
                                AuthRequestState::use_persistent_set(fermi_auth_request, AuthRequestState::default());
                            }
                        }
                    }
                }
            },
            None => {
                let mut query_pairs = form_urlencoded::parse(query_string.as_bytes());
                let code_pair = query_pairs.find(|(key, _value)| key == "code");
                match code_pair {
                    Some((_key, code)) => {
                        let auth_code = code.to_string();
                        let token_response_future = use_future(cx, (), |_| async move {
                            token_response(client, auth_code).await
                        });
                        match token_response_future.value() {
                            Some(token_response) => {
                                let id_token = token_response.id_token().unwrap();
                                AuthTokenState::use_persistent_set(fermi_auth_token, AuthTokenState {
                                    id_token: Some(id_token.clone()),
                                    refresh_token: token_response.refresh_token().cloned()
                                });

                                rsx! {
                                    div { "Log in successful, please wait" }
                                }
                            }
                            None => {
                                rsx! { div { "Signing in... Please wait" } }
                            }
                        }
                    }
                    None => {
                        rsx! { div { "No code provided" } }
                    }
                }
            }
        },
        None => {
            rsx!{{}}
        }
    })
}