# OpenID Connect example to show how to authenticate an user

The environment variables in [`.cargo/config.toml`](./.cargo/config.toml) must be set in order for this example to work.

Once they are set, you can run `dx serve --platform web` or `dx serve --platform desktop`.

### Environment variables summary

- `DIOXUS_FRONT_ISSUER_URL`: The openid-connect's issuer url
- `DIOXUS_FRONT_CLIENT_ID`: The openid-connect's client id
- `DIOXUS_FRONT_CLIENT_SECRET`: The openid-connect's client secret
- `DIOXUS_FRONT_URL`: The url the frontend is supposed to be running on, it could be for example `http://localhost:8080`
