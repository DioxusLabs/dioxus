# Renderização por Servidor

O Dioxus 'VirtualDom' pode ser renderizado por servidor.

[Exemplo: Dioxus DocSite](https://github.com/dioxusLabs/docsite)

## Suporte a Multitarefas

O Dioxus `VirtualDom`, infelizmente, atualmente não é `Send`. Internamente, usamos um pouco de mutabilidade interior que não é _thread-safe_. Isso significa que você não pode usar Dioxus facilmente com a maioria dos frameworks da web como Tide, Rocket, Axum, etc.

Para resolver isso, você deve gerar um `VirtualDom` em seu próprio thread e se comunicar com ele por meio de canais.

Ao trabalhar com frameworks web que requerem `Send`, é possível renderizar um `VirtualDom` imediatamente para uma `String` – mas você não pode manter o `VirtualDom` em um ponto de espera. Para SSR de estado retido (essencialmente LiveView), você precisará criar um _pool_ de `VirtualDoms`.

## Configurar

Se você quer apenas renderizar `rsx!` ou um VirtualDom para HTML, confira os documentos da API. É bem simples:

```rust
// We can render VirtualDoms
let mut vdom = VirtualDom::new(app);
let _ = vdom.rebuild();
println!("{}", dioxus::ssr::render_vdom(&vdom));

// Or we can render rsx! calls directly
println!( "{}", dioxus::ssr::render_lazy(rsx! { h1 { "Hello, world!" } } );
```

No entanto, para este guia, vamos mostrar como usar Dioxus SSR com `Axum`.

Certifique-se de ter o Rust and Cargo instalado e, em seguida, crie um novo projeto:

```shell
cargo new --bin demo
cd app
```

Adicione o Dioxus com o recurso `ssr`:

```shell
cargo add dioxus
cargo add dioxus-ssr
```

Em seguida, adicione todas as dependências do Axum. Isso será diferente se você estiver usando um Web Framework diferente

```
cargo add tokio --features full
cargo add axum
```

Suas dependências devem ficar mais ou menos assim:

```toml
[dependencies]
axum = "0.4.5"
dioxus = { version = "*" }
dioxus-ssr = { version = "*" }
tokio = { version = "1.15.0", features = ["full"] }
```

Agora, configure seu aplicativo Axum para responder em um _endpoint_.

```rust
use axum::{response::Html, routing::get, Router};
use dioxus::prelude::*;

#[tokio::main]
async fn main() {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on http://{}", addr);

    axum::Server::bind(&addr)
        .serve(
            Router::new()
                .route("/", get(app_endpoint))
                .into_make_service(),
        )
        .await
        .unwrap();
}
```

E, em seguida, adicione nosso _endpoint_. Podemos renderizar `rsx!` diretamente:

```rust
async fn app_endpoint() -> Html<String> {
    Html(dioxus_ssr::render_lazy(rsx! {
            h1 { "hello world!" }
    }))
}
```

Ou podemos renderizar `VirtualDoms`.

```rust
async fn app_endpoint() -> Html<String> {
    fn app(cx: Scope) -> Element {
        cx.render(rsx!(h1 { "hello world" }))
    }
    let mut app = VirtualDom::new(app);
    let _ = app.rebuild();

    Html(dioxus_ssr::render_vdom(&app))
}
```

E é isso!

> Você pode notar que não pode manter o VirtualDom em um ponto de espera. Dioxus atualmente não é ThreadSafe, então _deve_ permanecer no _thread_ que iniciou. Estamos trabalhando para flexibilizar essa exigência.
