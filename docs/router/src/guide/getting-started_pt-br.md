# Começando

Antes de começarmos a utilizar o Dioxus `Router`, precisamos inicializar um aplicativo da Web Dioxus.

#### Ferramentas necessárias

Se você ainda não o fez, certifique-se de instalar a ferramenta de compilação [dioxus-cli](https://dioxuslabs.com/nightly/cli/) e o alvo de compilação Rust `wasm32-unknown-unknown`:

```
$ cargo install dioxus-cli
    ...
$ rustup target add wasm32-unkown-unknown
    ...
```

### Criando o projeto

Primeiro, crie um novo projeto binário de carga:

```
cargo new --bin dioxus-blog
```

Em seguida, precisamos adicionar `dioxus` com o recurso "web" e `router` ao nosso arquivo `Cargo.toml`.

```toml
[package]
name = "dioxus-blog"
version = "0.1.0"
edition = "2021"

[dependencies]
dioxus = { version = "0.1.8", features = ["web", "router"] }
```

Agora podemos começar a codificar! Crie um arquivo `index.html` na raiz do seu projeto:

```html
<html>
  <head>
    <title>Dioxus Blog</title>
  </head>
  <body>
    <div id="main"></div>
  </body>
</html>
```

Você pode adicionar o que quiser neste arquivo, apenas certifique-se de ter um `div` com o id de `main` na raiz do seu elemento `body`. Este é essencialmente o identificador onde o Dioxus irá renderizar seus componentes.

Agora vá para `src/main.rs` e substitua seu conteúdo por:

```rs
use dioxus::prelude::*;

fn main() {
    // Launch Dioxus web app
    dioxus::web::launch(app);
}

// Our root component.
fn app(cx: Scope) -> Element {
    // Render "Hello, wasm!" to the screen.
    cx.render(rsx! {
        p { "Hello, wasm!"}
    })
}
```

Nosso projeto está pronto! Para garantir que tudo esteja funcionando corretamente, na raiz do seu projeto execute:

```
dioxus serve --platform web
```

Então vá para [http://localhost:8080](http://localhost:8080) em seu navegador, e você deverá ver `Hello, wasm!` na sua tela.

#### Conclusão

Montamos um novo projeto com o Dioxus e tudo funcionou corretamente. Em seguida, criaremos uma pequena página inicial e iniciaremos nossa jornada com o Dioxus `Router`.
