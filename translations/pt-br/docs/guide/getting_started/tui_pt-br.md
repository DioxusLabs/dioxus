# Primeiros passos: TUI

O suporte à `TUI` é atualmente bastante experimental. Até o nome do projeto mudará. Mas, se você estiver disposto a se aventurar no reino do desconhecido, este guia o ajudará a começar.

[Suporte TUI](https://github.com/DioxusLabs/rink/raw/master/examples/example.png)

## Configurando

Para mexer no suporte à `TUI`, comece criando um novo pacote e adicionando nosso recurso `TUI`.

```shell
$ cargo new --bin demo
$ cd demo
$ cargo add dioxus --features tui
```

Em seguida, edite seu `main.rs` com o modelo básico.

```rust
//  main
use dioxus::prelude::*;

fn main() {
    dioxus::tui::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",

            "Hello world!"
        }
    })
}
```

Para executar nosso aplicativo:

```shell
$ cargo run
```

Pressione "ctrl-c" para fechar o aplicativo. Para mudar de "ctrl-c" para apenas "q" para sair do programa, você pode iniciar o aplicativo com uma configuração para desativar a saída padrão e usar a raiz `TuiContext` para sair por conta própria.

```rust
use dioxus::events::{KeyCode, KeyboardEvent};
use dioxus::prelude::*;
use dioxus::tui::TuiContext;

fn main() {
    dioxus::tui::launch_cfg(
        app,
        dioxus::tui::Config::new()
            .without_ctrl_c_quit()
            // Some older terminals only support 16 colors or ANSI colors if your terminal is one of these change this to BaseColors or ANSI
            .with_rendering_mode(dioxus::tui::RenderingMode::Rgb),
    );
}

fn app(cx: Scope) -> Element {
    let tui_ctx: TuiContext = cx.consume_context().unwrap();

    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",
            onkeydown: move |k: KeyboardEvent| if let KeyCode::Q = k.data.key_code {
                tui_ctx.quit();
            },

            "Hello world!"
        }
    })
}
```

## Notas

- Nosso pacote `TUI` usa `flexbox` para layout
- `1px` é a altura da linha de um caractere. Seu `px` de CSS regular não é traduzido.
- Se seu aplicativo entrar em pânico, seu terminal será destruído. Isso será corrigido eventualmente.

## Passos Futuros

Certifique-se de ler o [Guia Dioxus](https://dioxuslabs.com/guide) se você ainda não leu!
