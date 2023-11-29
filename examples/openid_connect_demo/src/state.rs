//! OIDC Auth implemented as a Fermi atom
//!
//!
//!
//!
//!

use crate::constants::*;
use anyhow::Context;
use fermi::*;
use gloo_storage::{LocalStorage, Storage};
use openidconnect::{
    core::{CoreClient, CoreIdToken, CoreIdTokenClaims, CoreResponseType},
    reqwest::async_http_client,
    AuthenticationFlow, AuthorizationCode, ClaimsVerificationError, ClientId, CsrfToken, IssuerUrl,
    LogoutRequest, Nonce, OAuth2TokenResponse, ProviderMetadataWithLogout, RedirectUrl,
    RefreshToken, TokenResponse,
};
use serde::{Deserialize, Serialize};

pub static USER: Atom<Auth> = Atom::new(|_| Auth::load());

/// The auth state of the app.
///
/// This allows places
pub struct Auth {
    // A plausible OIDC client formed via discovery
    client: Option<CoreClient>,

    /// The OIDC provider metadata, saved alongside the client to generate the logout url
    metadata: Option<ProviderMetadataWithLogout>,

    /// The ID token that was returned by the OIDC provider
    token: Option<Token>,

    /// The OIDC request that was used to generate the auth url
    active_request: Option<AuthRequest>,
}

#[derive(Serialize, Deserialize)]
struct Token {
    id_token: CoreIdToken,
    refresh_token: RefreshToken,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AuthRequest {
    pub nonce: Nonce,
    pub authorize_url: String,
}

impl Auth {
    fn load() -> Self {
        Self {
            token: LocalStorage::get(DIOXUS_FRONT_AUTH_TOKEN).ok(),
            active_request: LocalStorage::get(DIOXUS_FRONT_AUTH_REQUEST).ok(),
            metadata: None,
            client: None,
        }
    }

    pub async fn load_client() -> anyhow::Result<()> {
        // Discover the OIDC provider
        let metadata = init_provider_metadata().await?;

        // Load the client
        let client = init_oidc_client(metadata.clone()).await?;

        // Try and refresh the token if we can
        USER.write().client = Some(client);
        USER.write().metadata = Some(metadata);

        // If the token is expired, refresh it
        if let Some(Err(ClaimsVerificationError::Expired(_))) = USER().verify_claims() {
            refresh_session().await?;
        };

        Ok(())
    }

    pub fn logged_in(&self) -> bool {
        self.token.is_some()
    }

    pub fn profile(&self) -> Option<String> {
        todo!()
    }

    pub fn login_token(&self) -> Option<&CoreIdToken> {
        self.token.as_ref().map(|t| &t.id_token)
    }

    pub fn refresh_token(&self) -> Option<&RefreshToken> {
        self.token.as_ref().map(|t| &t.refresh_token)
    }

    /// Get the email of the user
    ///
    /// This might fail if the token is expired. We might want to try and manually refresh the token, however, it's a
    /// simpler design to just refresh the token when the API gives a permissions error instead.
    ///
    /// Instead, we might want to cache the email in local storage, and use that if the token is expired.
    pub fn email(&self) -> Option<String> {
        Some(
            self.verify_claims()?
                .ok()?
                .email()
                .cloned()
                .expect("No email in id token")
                .to_string(),
        )
    }

    /// Verify that the token is valid
    fn verify_claims(&self) -> Option<Result<&CoreIdTokenClaims, ClaimsVerificationError>> {
        let id_token = self.token.as_ref()?;
        let client = self.client.as_ref()?;
        let nonce = &self.active_request.as_ref()?.nonce;
        Some(id_token.id_token.claims(&client.id_token_verifier(), nonce))
    }

    /// Convert an authorization code into an access token
    ///
    /// When the user is redirected back to the app from the OIDC provider, the query string will contain an authorization code.
    /// However, this code is not the access token. Instead, it must be exchanged for an access token. This means we need
    /// to go back to the OIDC provider and make a request to exchange the code for a token.
    ///
    /// This function does that, and stores the token in the app's state.
    pub async fn exchange_code(code: String) -> anyhow::Result<()> {
        let token_response = USER()
            .owned_client()?
            .exchange_code(AuthorizationCode::new(code))
            .request_async(async_http_client)
            .await?;

        USER.write().token = Some(Token {
            id_token: token_response.id_token().cloned().unwrap(),
            refresh_token: token_response.refresh_token().cloned().unwrap(),
        });

        Ok(())
    }

    fn owned_client(&self) -> anyhow::Result<CoreClient> {
        self.client.as_ref().cloned().context("No client")
    }

    pub fn logout(&mut self) {
        self.token = None;
        self.client = None;
        LocalStorage::delete(DIOXUS_FRONT_AUTH_TOKEN);
        LocalStorage::delete(DIOXUS_FRONT_AUTH_REQUEST);
    }

    /// Open the login page on the OIDC provider
    pub fn login(&mut self) {
        let Some(client) = self.client.as_ref() else {
            return;
        };

        let (authorize_url, _csrf_state, nonce) = client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(openidconnect::Scope::new("email".to_string()))
            .add_scope(openidconnect::Scope::new("profile".to_string()))
            .url();

        let auth_request = AuthRequest {
            authorize_url: authorize_url.to_string(),
            nonce,
        };

        LocalStorage::set(DIOXUS_FRONT_AUTH_REQUEST, &auth_request).unwrap();

        self.active_request = Some(auth_request);
    }

    pub fn login_url(&self) -> Option<String> {
        self.active_request
            .as_ref()
            .map(|r| r.authorize_url.clone())
    }

    pub fn logout_url(&self) -> Option<String> {
        let id_token_hint = self.login_token()?;
        let provider_metadata = self.metadata.as_ref()?;

        let end_session_url: LogoutRequest = provider_metadata
            .additional_metadata()
            .clone()
            .end_session_endpoint
            .unwrap()
            .into();

        let request = end_session_url
            .set_client_id(ClientId::new(DIOXUS_FRONT_CLIENT_ID.to_string()))
            .set_id_token_hint(id_token_hint)
            .http_get_url();

        Some(request.to_string())
    }
}

async fn refresh_session() -> Result<(), anyhow::Error> {
    let refresh_token = USER().refresh_token().context("No refresh token")?.clone();

    let token_response = USER()
        .owned_client()?
        .exchange_refresh_token(&refresh_token)
        .request_async(async_http_client)
        .await?;

    USER.write().token = Some(Token {
        id_token: token_response.id_token().cloned().unwrap(),
        refresh_token: token_response.refresh_token().cloned().unwrap(),
    });
    Ok(())
}

pub async fn init_oidc_client(
    provider_metadata: ProviderMetadataWithLogout,
) -> anyhow::Result<CoreClient> {
    let client_id = ClientId::new(crate::constants::DIOXUS_FRONT_CLIENT_ID.to_string());
    let client_secret = None;
    let redirect_url = RedirectUrl::new(format!("{}/login", crate::constants::DIOXUS_FRONT_URL))?;

    Ok(
        CoreClient::from_provider_metadata(provider_metadata, client_id, client_secret)
            .set_redirect_uri(redirect_url),
    )
}

pub async fn init_provider_metadata() -> anyhow::Result<ProviderMetadataWithLogout> {
    let issuer_url = IssuerUrl::new(crate::constants::DIOXUS_FRONT_ISSUER_URL.to_string())?;
    Ok(ProviderMetadataWithLogout::discover_async(issuer_url, async_http_client).await?)
}
