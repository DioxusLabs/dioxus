# Introdução: renderização no lado do servidor

O Dioxus `VirtualDom` pode ser renderizado em uma string percorrendo a árvore de elementos. Isso é implementado na `crate` `ssr` onde seu aplicativo Dioxus pode ser renderizado diretamente em HTML para ser servido por um servidor Web.

## Configurando

Se você quer apenas renderizar `rsx!` ou um `VirtualDom` para HTML, confira os documentos da API. É bem simples:

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
$ cargo new --bin demo
$ cd app
```

Adicione o Dioxus com o recurso `ssr`:

```shell
$ cargo add dioxus --features ssr
```

Em seguida, adicione todas as dependências do `Axum`. Isso será diferente se você estiver usando um Web Framework diferente

```
$ cargo add tokio --features full
$ cargo add axum
```

Suas dependências devem ficar mais ou menos assim:

```toml
[dependencies]
axum = "0.4.5"
dioxus = { version = "*", features = ["ssr"] }
tokio = { version = "1.15.0", features = ["full"] }
```

Agora, configure seu aplicativo `Axum` para responder em um "endpoint".

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

E, em seguida, adicione nosso endpoint. Podemos renderizar `rsx!` diretamente:

```rust
async fn app_endpoint() -> Html<String> {
    Html(dioxus::ssr::render_lazy(rsx! {
            h1 { "hello world!" }
    }))
}
```

Ou podemos renderizar `VirtualDoms`:

```rust
async fn app_endpoint() -> Html<String> {
    fn app(cx: Scope) -> Element {
        cx.render(rsx!(h1 { "hello world" }))
    }
    let mut app = VirtualDom::new(app);
    let _ = app.rebuild();

    Html(dioxus::ssr::render_vdom(&app))
}
```

E é isso!

> Você pode notar que não pode manter o `VirtualDom` em um `await`. O Dioxus atualmente não é `ThreadSafe`, então _deve_ permanecer no thread que iniciou. Estamos trabalhando para corrigir essa exigência.

## Passos Futuros

Certifique-se de ler o [Guia Dioxus](https://dioxuslabs.com/guide) se você ainda não leu!
