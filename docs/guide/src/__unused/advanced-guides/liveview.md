# Dioxus Liveview
Liveview is a configuration where a server and a client work together to render a Dioxus app. Liveview monomorphizes a web application, eliminating the need for frontend-specific APIs.

This is a developer-friendly alternative to the JAM-stack (Javascript + API + Markdown), combining the Wasm-compatibility and async performance of Rust.

## Why liveview?

### No APIs necessary!
Because Liveview combines the server and the client, you'll find dedicated APIs unnecessary. You'll still want to implement a data-fetching service for Live-apps, but this can be implemented as a crate and shared between apps. This approach was designed to let you model out your data requirements without needing to maintain a public versioned API.

You can find more information to data modeling and fetching for LiveApps in the "Book of Dioxus Patterns".

### Ease of deployment
There is no hassle for deploying Dioxus liveview apps - simply upload the binary to your hosting provider of choice. There simply is no need to deal with intermediate APIs. An app's frontend and backend are tightly coupled to each other, meaning that the backed and frontend are always up to date.

This is especially powerful for small teams, where fast iteration, versioned unique prototypes, and simple deployments are important. As your app grows, Dioxus will happily grow with you, serving up to 100K RPS (thanks to async Rust performance).

### Power of server rendering combined with the reactive nature of JavaScript
Liveview apps are backed by a server and enhanced with web-assembly. This completely eliminates the need to write Javascript to add dynamic content to webpages. Apps are written in only **one** language and require essentially 0 knowledge of build systems, transpiling or ECMAScript versions. Minimum browser support is guaranteed to cover 95% of users, and the Dioxus-CLI can be configured to generate polyfills for even more support.

### Support with LiveHost by Dioxus-Labs
The Dioxus-CLI comes ready for use with premium Dioxus-Labs services, like LiveHost. Simply link up your git repo to the Dioxus LiveHost and start deploying your fullstack Dioxus app today. LiveHost supports:

- Versioned fronted/backend with unique links
- Builtin CI/CD
- Managed and pluggable storage database backends
- Serverless support for minimal latency
- Site Analytics
- Lighthouse optimization
- On-premise support (see license terms)

Dioxus LiveHost is a Dioxus LiveApp management service with all supported features in a single binary. For OSS, we permit free usage and deployment of LiveHost to your favorite hosting provider.
