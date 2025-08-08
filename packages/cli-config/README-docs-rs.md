<div align="center">
 <img
 src="https://github.com/user-attachments/assets/6c7e227e-44ff-4e53-824a-67949051149c"
 alt="Build web, desktop, and mobile apps with a single codebase."
 width="100%"
 class="darkmode-image"
 >
</div>

# Dioxus CLI configuration

This crate exposes the various configuration options that the Dioxus CLI sets when running the
application during development.

Note that these functions will return a different value when running under the CLI, so make sure
not to rely on them when running in a production environment.

## Constants

The various constants here are the names of the environment variables that the CLI sets. We recommend
using the functions in this crate to access the values of these environment variables indirectly.

The CLI uses this crate and the associated constants to _set_ the environment variables, but as
a consumer of the CLI, you would want to read the values of these environment variables using
the provided functions.

## Example Usage

We recommend using the functions here to access the values of the environment variables set by the CLI.
For example, you might use the [`fullstack_address_or_localhost`] function to get the address that
the CLI is requesting the application to be served on.

```rust, ignore
async fn launch_axum(app: axum::Router<()>) {
// Read the PORT and ADDR environment variables set by the CLI
let addr = dioxus_cli_config::fullstack_address_or_localhost();

     // Bind to the address and serve the application
     let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
     axum::serve(listener, app.into_make_service())
         .await
         .unwrap();

}
```

## Stability

The _values_ that these functions return are _not_ guaranteed to be stable between patch releases
of Dioxus. At any time, we might change the values that the CLI sets or the way that they are read.

We also don't guarantee the stability of the env var names themselves. If you want to rely on a
particular env var, use the defined constants in your code.
