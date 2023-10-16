# OpenID Connect example to show how to authenticate an user

The environment variables in  `.cargo/config.toml` must be set in order for this example to work(if this example is just being compiled from the root workspace, the `.cargo/config.toml` from the root workspace must be set as stated in the [Cargo book](https://doc.rust-lang.org/cargo/reference/config.html)).

Once they are set, you can run `dx serve`

### Environment variables summary

```DIOXUS_FRONT_ISSUER_URL``` The openid-connect's issuer url 

```DIOXUS_FRONT_CLIENT_ID``` The openid-connect's client id

```DIOXUS_FRONT_URL``` The url the frontend is supposed to be running on, it could be for example `http://localhost:8080`