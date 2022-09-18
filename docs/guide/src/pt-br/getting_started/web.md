# Web

Crie aplicativos de página única (SPA) que são executados no navegador com o Dioxus. Para rodar na Web, seu aplicativo deve ser compilado para WebAssembly e utilizar a `crate` `dioxus` com o recurso `web` ativado.

Uma compilação do Dioxus para a web será aproximadamente equivalente ao tamanho de uma compilação do React (70kb vs 65kb), mas carregará significativamente mais rápido devido ao [StreamingCompile do WebAssembly](https://hacks.mozilla.org/2018/01/making-webassembly-even-faster-firefoxs-new-streaming-and-tiering-compiler/).

Exemplos:

- [TodoMVC](https://github.com/DioxusLabs/example-projects/tree/master/todomvc)
- [ECommerce](https://github.com/DioxusLabs/example-projects/tree/master/ecommerce-site)

[![Exemplo de TodoMVC](https://github.com/DioxusLabs/example-projects/raw/master/todomvc/example.png)](https://github.com/DioxusLabs/example-projects/blob/master /todomvc)

> Nota: Devido às limitações do Wasm, nem todos as `crates` funcionarão com seus aplicativos da web, portanto, você precisará certificar-se de que suas `crates` funcionem sem chamadas de sistema nativas (temporizadores, IO, etc.).

## Suporte

A Web é a plataforma de destino com melhor suporte para Dioxus.

## Ferramentas

Para desenvolver seu aplicativo Dioxus para a web, você precisará de uma ferramenta para construir e servir seus itens. Recomendamos usar [trunk](https://trunkrs.dev) que inclui um sistema de compilação, otimização Wasm, um servidor dev e suporte para SASS/CSS:

```shell
cargo install trunk
```

Certifique-se de que o destino `wasm32-unknown-unknown` esteja instalado:

```shell
rustup target add wasm32-unknown-unknown
```

## Criando um Projeto

Crie uma nova caixa:

```shell
cargo new --bin demo
cd demo
```

Adicione o Dioxus como uma dependência com o recurso `web`:

```bash
cargo add dioxus --features web
```

Adicione um `index.html` para o `Trunk` usar. Certifique-se de que seu elemento "mount point" tenha um ID de "main":

```html
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  </head>
  <body>
    <div id="main"></div>
  </body>
</html>
```

Edite seu `main.rs`:

```rust
{{#include ../../../examples/hello_world_web.rs}}
```

E para servir nosso aplicativo:

```bash
trunk serve
```
