# Primeiros passos: Desktop

Um dos recursos fortes do Dioxus é a capacidade de criar rapidamente um aplicativo de desktop nativo que parece e se comporta o mesmo em todas as plataformas. Os aplicativos criados com o Dioxus geralmente têm menos de 5 MB de tamanho e usam os recursos existentes do sistema, para que não consumam quantidades enormes de RAM.

Dioxus Desktop é construído a partir do `Tauri`. No momento, não há abstrações do Dioxus para atalhos de teclado, barra de menus, manuseio, etc., então você deve aproveitar o `Tauri` - principalmente [Wry](http://github.com/tauri-apps/wry/) e [ Tao](http://github.com/tauri-apps/tao)) diretamente. A próxima grande versão do Dioxus-Desktop incluirá componentes e hooks para notificações, atalhos globais, barra de menus, etc.

## Configurando

Configurar o Dioxus-Desktop é bastante fácil. Certifique-se de ter o Rust and Cargo instalado e, em seguida, crie um novo projeto:

```shell
$ cargo new --bin demo
$ cd demo
```

Adicione o Dioxus com o recurso `desktop`:

```shell
$ cargo add dioxus --features desktop
```

Edite seu `main.rs`:

```rust
// main.rs
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        div {
            "hello world!"
        }
    })
}
```

Para configurar a visualização da web, a barra de menus e outros recursos importantes específicos da área de trabalho, confira algumas das configurações de inicialização na [referência da API](https://docs.rs/dioxus-desktop/).

## Futuros Passos

Certifique-se de ler o [Guia Dioxus](https://dioxuslabs.com/guide) se você ainda não leu!
