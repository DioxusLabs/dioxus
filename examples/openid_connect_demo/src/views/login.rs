use crate::{
    env::DIOXUS_FRONT_URL,
    oidc::{token_response, AuthRequestState, AuthTokenState},
    router::Route,
    storage::PersistentWrite,
    FERMI_AUTH_REQUEST, FERMI_AUTH_TOKEN, FERMI_CLIENT,
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
    let client = fermi_client.read().oidc_client.clone();
    let auth_token_read = fermi_auth_token.read().clone();
    cx.render(match (client, auth_token_read) {
        (Some(client), Some(auth_token_read)) => {
            match (auth_token_read.id_token, auth_token_read.refresh_token) {
                (Some(_id_token), Some(_refresh_token)) => {
                    rsx! {
                        div{"Sign in successful"}
                        Link{
                            to: home_url,"Go back home"
                        }
                    }
                }
                // If the refresh token is set but not the id_token, there was an error, we just go back home and reset their value
                (None, Some(_)) | (Some(_), None) => {
                    rsx! {
                        div{"Error while attempting to log in"}
                        Link{
                            to: home_url, "Go back home", onclick: move |_|{
                                AuthTokenState::persistent_set(fermi_auth_token, Some(AuthTokenState::default()));
                                AuthRequestState::persistent_set(fermi_auth_request, Some(AuthRequestState::default()));
                            }
                        }
                    }
                }
                (None, None) => {
                    let mut query_pairs = form_urlencoded::parse(query_string.as_bytes());
                let code_pair = query_pairs.find(|(key, _value)| key == "code");
                match code_pair {
                    Some((_key, code)) => {
                        let auth_code = code.to_string();
                        let token_response_spawn = move ||{
                            cx.spawn({
                                let fermi_auth_token = fermi_auth_token.to_owned();
                                async move {
                                    let token_response = token_response(client, auth_code).await;
                                    let id_token = token_response.id_token().unwrap();
                                    AuthTokenState::persistent_set(&fermi_auth_token, Some(AuthTokenState {
                                        id_token: Some(id_token.clone()),
                                        refresh_token: token_response.refresh_token().cloned()
                                }));
                                }
                            })
                        };
                        token_response_spawn();
                        rsx!{div{}}
                    }
                    None => {
                        rsx! { div { "No code provided" } }
                    }
                }
                }
            }
        }
        (_, _) => {
            rsx! {{}}
        }
    })
}
