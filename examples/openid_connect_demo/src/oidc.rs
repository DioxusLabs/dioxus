use anyhow::Result;
use openidconnect::{
    core::{CoreClient, CoreIdToken, CoreResponseType, CoreTokenResponse},
    reqwest::async_http_client,
    url::Url,
    AuthenticationFlow, AuthorizationCode, ClaimsVerificationError, ClientId, ClientSecret,
    CsrfToken, IssuerUrl, LogoutRequest, Nonce, ProviderMetadataWithLogout, RedirectUrl,
    RefreshToken,
};
use serde::{Deserialize, Serialize};

use crate::{props::client::ClientProps, DIOXUS_FRONT_CLIENT_ID};

#[derive(Clone, Debug, Default)]
pub struct ClientState {
    pub oidc_client: Option<ClientProps>,
}

/// State that holds the nonce and authorization url and the nonce generated to log in an user
#[derive(Clone, PartialEq, Deserialize, Serialize, Default)]
pub struct AuthRequestState {
    pub auth_request: Option<AuthRequest>,
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct AuthRequest {
    pub nonce: Nonce,
    pub authorize_url: String,
}

/// State the tokens returned once the user is authenticated
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct AuthTokenState {
    /// Token used to identify the user
    pub id_token: Option<CoreIdToken>,
    /// Token used to refresh the tokens if they expire
    pub refresh_token: Option<RefreshToken>,
}

impl PartialEq for AuthTokenState {
    fn eq(&self, other: &Self) -> bool {
        self.id_token == other.id_token
            && self.refresh_token.as_ref().map(|t| t.secret().clone())
                == other.refresh_token.as_ref().map(|t| t.secret().clone())
    }
}

pub fn email(
    client: CoreClient,
    id_token: CoreIdToken,
    nonce: Nonce,
) -> Result<String, ClaimsVerificationError> {
    match id_token.claims(&client.id_token_verifier(), &nonce) {
        Ok(claims) => Ok(claims.clone().email().unwrap().to_string()),
        Err(error) => Err(error),
    }
}

pub fn authorize_url(client: CoreClient) -> AuthRequest {
    let (authorize_url, _csrf_state, nonce) = client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(openidconnect::Scope::new("email".to_string()))
        .add_scope(openidconnect::Scope::new("profile".to_string()))
        .url();
    AuthRequest {
        authorize_url: authorize_url.to_string(),
        nonce,
    }
}

pub async fn init_provider_metadata() -> Result<ProviderMetadataWithLogout> {
    let issuer_url = IssuerUrl::new(crate::DIOXUS_FRONT_ISSUER_URL.to_string())?;
    Ok(ProviderMetadataWithLogout::discover_async(issuer_url, async_http_client).await?)
}

pub async fn init_oidc_client() -> Result<(ClientId, CoreClient)> {
    let client_id = ClientId::new(crate::DIOXUS_FRONT_CLIENT_ID.to_string());
    let provider_metadata = init_provider_metadata().await?;
    let client_secret = Some(ClientSecret::new(
        crate::DIOXUS_FRONT_CLIENT_SECRET.to_string(),
    ));
    let redirect_url = RedirectUrl::new(format!("{}/login", crate::DIOXUS_FRONT_URL))?;

    Ok((
        client_id.clone(),
        CoreClient::from_provider_metadata(provider_metadata, client_id, client_secret)
            .set_redirect_uri(redirect_url),
    ))
}

///TODO: Add pkce_pacifier
pub async fn token_response(oidc_client: CoreClient, code: String) -> Result<CoreTokenResponse> {
    // let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    Ok(oidc_client
        .exchange_code(AuthorizationCode::new(code.clone()))
        // .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await?)
}

pub async fn exchange_refresh_token(
    oidc_client: CoreClient,
    refresh_token: RefreshToken,
) -> Result<CoreTokenResponse> {
    Ok(oidc_client
        .exchange_refresh_token(&refresh_token)
        .request_async(async_http_client)
        .await?)
}

pub async fn log_out_url(id_token_hint: CoreIdToken) -> Result<Url> {
    let provider_metadata = init_provider_metadata().await?;
    let end_session_url = provider_metadata
        .additional_metadata()
        .clone()
        .end_session_endpoint
        .unwrap();
    let logout_request: LogoutRequest = LogoutRequest::from(end_session_url);
    Ok(logout_request
        .set_client_id(ClientId::new(DIOXUS_FRONT_CLIENT_ID.to_string()))
        .set_id_token_hint(&id_token_hint)
        .http_get_url())
}
