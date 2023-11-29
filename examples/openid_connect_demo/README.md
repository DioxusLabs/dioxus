# OpenID Connect example to show how to authenticate an user

## What is OpenID connect?

OpenID Connect makes it possible for developers to add auth to their web applications by utilizing an open standard called OIDC.

OIDC standardizes the necessary bits of information and format required to authenticate users with OAuth providers like Auth0.

Read more here:

https://openidconnect.net/introduction


## How is it implemented in this Dioxus example?

In this example, we combine a few concepts to provide a basic skeleton of an app that implements auth using any OIDC compatible provider. Specifically, we showcase how to integrate auth using Fermi for state management and Dioxus Router for handling redirects and guards.

1. When the app is rendered for the first time, we try and load the auth token from local storage.
2. If no token exists, we render a "login" button and populate the global token with `None`
3. When the user clicks "login", we initiate a discovery request to OIDC
4. OIDC will respond with a login splash screen. The user logs in.
5. The OIDC provider then hits our app's `login` endpoint with a token in the URL
6. We need to hit the OIDC provider again with that access token to get the actual auth token
7. We cache the token into Fermi and local storage.
8. The app can now render appropriately with the token cached
9. A logout button is now present that clears local storage and the cache

## Token Refresh Behavior

OIDC works by exchanging a long-lived access token for a shorter-lived auth token. This ensures that we can quickly revoke auth and that sessions don't last long enough to be compromised. It also limits the window of interception that an attacker might use to capture a session.

To limit how long valid tokens are out in the wild, the OIDC provider actually provides *two* tokens - the auth token and a refresh token. The client can check if the auth token is still valid by verifying its claims with a verification provider. If the token is expired, it can refresh it at any time.

This example is structured to try and refresh the token if it's expired, but this is not necessary. An alternative approach would be to attempt transactions with expired auth tokens and then, if they fail, refresh the session with the refresh token. This has the benefit of simplified logic around auth handling but the con of expired requests taking longer.

## Configuring this example for any provider:


The environment variables in  `.cargo/config.toml` must be set in order for this example to work(if this example is just being compiled from the root workspace, the `.cargo/config.toml` from the root workspace must be set as stated in the [Cargo book](https://doc.rust-lang.org/cargo/reference/config.html)).

Once they are set, you can run `dx serve`

### Environment variables summary

```DIOXUS_FRONT_ISSUER_URL``` The openid-connect's issuer url

```DIOXUS_FRONT_CLIENT_ID``` The openid-connect's client id

```DIOXUS_FRONT_URL``` The url the frontend is supposed to be running on, it could be for example `http://localhost:8080`

## Running without an auth server

By default, if no providers are configured, we use the OIDC playground.

https://openidconnect.net/#
