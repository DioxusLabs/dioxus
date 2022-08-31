# IU do terminal

Você pode construir uma interface baseada em texto que será executada no terminal usando o Dioxus.

![Hello World screenshot](https://github.com/DioxusLabs/rink/raw/master/examples/example.png)

> Nota: este livro foi escrito tendo em mente plataformas baseadas em HTML. Você pode acompanhar a TUI, mas terá que se adaptar um pouco.

## Suporte

O suporte à TUI é atualmente bastante experimental. Até o nome do projeto mudará. Mas, se você estiver disposto a se aventurar no reino do desconhecido, este guia o ajudará a começar.

## Configurando

Comece criando um novo pacote e adicionando nosso recurso TUI.

```shell
cargo new --bin demo
cd demo
cargo add dioxus --features tui
```

Em seguida, edite seu `main.rs` com o modelo básico.

```rust
{{#include ../../../examples/hello_world_tui.rs}}
```

Para executar nosso aplicativo:

```shell
cargo run
```

Pressione "ctrl-c" para fechar o aplicativo. Para mudar de "ctrl-c" para apenas "q" para sair, você pode iniciar o aplicativo com uma configuração para desativar o sair padrão e usar a raiz TuiContext para sair por conta própria.

```rust
{{#include ../../../examples/hello_world_tui_no_ctrl_c.rs}}
```

## Notas

- Nosso pacote TUI usa flexbox para layout
- 1px é a altura da linha de um caractere. Seu px CSS regular não traduz.
- Se seu aplicativo entrar em pânico, seu terminal será destruído. Isso será corrigido eventualmente.
