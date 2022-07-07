# Primeiros passos: celular

O Dioxus é único porque realmente suporta dispositivos móveis. No entanto, o suporte é muito jovem e você pode precisar mergulhar em alguns dos primitivos até que um suporte melhor esteja pronto.

Atualmente, apenas iOS é suportado por nós, mas você _pode_ adicionar suporte ao Android seguindo as mesmas instruções abaixo usando o guia `android` em `cargo-mobile`.

Além disso, o Dioxus Desktop e o Dioxus Mobile compartilham a mesma base de código, e o dioxus-mobile atualmente apenas reexporta o dioxus-desktop.

## Configurando

A configuração com dispositivos móveis pode ser bastante desafiadora. As ferramentas aqui não são ótimas (ainda) e podem levar alguns hacks para fazer as coisas funcionarem. O macOS M1 é amplamente inexplorado e pode não funcionar para você.

Vamos usar `cargo-mobile` para construir para dispositivos móveis. Primeiro, instale-o:

```shell
$ cargo install --git https://github.com/BrainiumLLC/cargo-mobile
```

E, em seguida, inicialize seu aplicativo para a plataforma certa. Use o modelo `winit` por enquanto. No momento, não há modelo "Dioxus" no `cargo-mobile`.

```shell
$ cargo mobile init
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
dioxus = { version = "*", features = ["mobile"] }
simple_logger = "*"
```

Edit your `lib.rs`:

```rust
// main.rs
use dioxus::prelude::*;

fn main() {
    dioxus::mobile::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        div {
            "hello world!"
        }
    })
}
```

Para configurar a visualização da web, a barra de menus e outros recursos importantes específicos da área de trabalho, confira algumas das configurações de inicialização na [referência da API](https://docs.rs/dioxus-mobile/).

## Passos Futuros

Certifique-se de ler o [Guia Dioxus](https://dioxuslabs.com/guide) se você ainda não leu!
