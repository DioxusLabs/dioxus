# Aplicativo móvel

Crie um aplicativo móvel com Dioxus!

Exemplo: [Aplicativo Todo](https://github.com/DioxusLabs/example-projects/blob/master/ios_demo)

## Suporte

Atualmente, a plataforma móvel é o destino de renderizador menos suportado para o Dioxus. Os aplicativos móveis são renderizados com o WebView da plataforma, o que significa que animações, transparência e widgets nativos não estão disponíveis no momento.

Além disso, o iOS é a única plataforma móvel compatível. É possível obter o Dioxus rodando no Android e renderizado com WebView, mas a biblioteca de janelas do Rust que o Dioxus usa – tao – atualmente não suporta Android.

Atualmente, o suporte móvel é mais adequado para aplicativos no estilo CRUD, idealmente para equipes internas que precisam desenvolver rapidamente, mas não se importam muito com animações ou widgets nativos.

## Configurando

A configuração com dispositivos móveis pode ser bastante desafiadora. As ferramentas aqui não são ótimas (ainda) e podem precisar de alguns _hacks_ para fazer as coisas funcionarem. O macOS M1 é amplamente inexplorado e pode não funcionar para você.

Vamos usar `cargo-mobile` para construir para dispositivos móveis. Primeiro, instale-o:

```shell
cargo install --git https://github.com/BrainiumLLC/cargo-mobile
```

E, em seguida, inicialize seu aplicativo para a plataforma certa. Use o modelo `winit` por enquanto. No momento, não há modelo "Dioxus" no cargo-mobile.

```shell
cargo mobile init
```

Nós vamos limpar completamente as `dependências` que ele gera para nós, trocando `winit` por `dioxus-mobile`.

```toml

[package]
name = "dioxus-ios-demo"
version = "0.1.0"
authors = ["Jonathan Kelley <jkelleyrtp@gmail.com>"]
edition = "2018"


# leave the `lib` declaration
[lib]
crate-type = ["staticlib", "cdylib", "rlib"]


# leave the binary it generates for us
[[bin]]
name = "dioxus-ios-demo-desktop"
path = "gen/bin/desktop.rs"

# clear all the dependencies
[dependencies]
mobile-entry-point = "0.1.0"
dioxus = { version = "*" }
dioxus-desktop = { version = "*" }
simple_logger = "*"
```

Edite seu `lib.rs`:

```rust
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        div {
            "hello world!"
        }
    })
}
```
