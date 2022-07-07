# Novo aplicativo

Para começar, vamos criar um novo projeto Rust para nosso Dog Search Engine.

```shell
$ cargo new --bin doggo
$ cd doggo
```

Certifique-se de que nosso projeto seja compilado por padrão

```shell
$ cargo run

   Compiling doggo v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 0.41s
     Running `target/debug/doggo`
Hello, world!
```

## Adicionando o Dioxus Desktop como uma dependência

Podemos editar nosso Cargo.toml diretamente:

```toml
[dependencies]
dioxus = { version = "*", features = ["desktop"]}
```

or use `cargo-edit` to add it via the CLI:

```shell
$ cargo add dioxus --features desktop
```

## Configurando um olá mundo

Vamos editar o `main.rs` do projeto e adicionar o esqueleto do

```rust
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(
        div { "hello world!" }
    ))
}
```

## Making sure things run
