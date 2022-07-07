# Introdução: Web

[_"Arrume suas coisas, vamos em uma aventura!"_](https://trunkrs.dev)

Dioxus é bem suportado para a Web através do `WebAssembly`. Uma compilação do Dioxus para a Web será aproximadamente equivalente ao tamanho de uma compilação do React (70kb vs 65kb), mas carregará significativamente mais rápido devido ao [StreamingCompile do WebAssembly](https://hacks.mozilla.org/2018/01/making-webassembly-even-faster-firefoxs-new-streaming-and-tiering-compiler/).

A criação de aplicativos Dioxus para a Web requer muito menos configuração do que nossas contrapartes JavaScript.

## Ferramentas

Para desenvolver seu aplicativo Dioxus para a Web, você precisará de uma ferramenta para construir e servir seus items. Recomendamos usar [trunk](https://trunkrs.dev) que inclui um sistema de compilação, otimização Wasm, um servidor dev e suporte para SASS/CSS.

Atualmente, os projetos de `trunk` só podem construir o binário raiz (ou seja, o `main.rs`). Para construir um novo projeto compatível com Dioxus, isso deve colocá-lo em funcionamento:

Primeiro, [instale o trunk](https://trunkrs.dev/#install):

```shell
$ cargo install trunk
```

Make sure the `wasm32-unknown-unknown` target is installed:

```shell
$ rustup target add wasm32-unknown-unknown
```

Crie um novo projeto:

```shell
$ cargo new --bin demo
$ cd demo
```

Adicione o Dioxus com os recursos `web`:

```
$ cargo add dioxus --features web
```

Adicione um `index.html` para o `Trunk` usar. Certifique-se de que seu elemento "mount point" tenha um ID de "main" (isso pode ser configurado posteriormente):

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

Edit seu `main.rs`:

```rust
// main.rs

use dioxus::prelude::*;

fn main() {
    dioxus::web::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        div { "hello, wasm!" }
    })
}
```

E para servir nosso aplicativo:

```shell
trunk serve
```

Para construir nosso aplicativo e publicá-lo no Github:

- Verifique se o Github Pages está configurado para seu repositório
- Crie seu aplicativo com `trunk build --release` (inclua `--public-url <repo-name>` para atualizar os prefixos de ativos se estiver usando um site de projeto)
- Mova seu HTML/CSS/JS/Wasm gerado de `dist` para a pasta configurada para Github Pages
- Adicione e confirme com git
- `push` para o Github

É isso!

## Ferramenta de Construção Futura

Atualmente, estamos trabalhando em nossa própria ferramenta de compilação chamada [Dioxus Studio](http://github.com/dioxusLabs/studio) que suportará:

- uma TUI interativa
- reconfiguração em tempo real
- recarga de CSS em tempo-real
- link de dados bidirecional entre o navegador e o código-fonte
- um interpretador para `rsx!`
- capacidade de publicar no github/netlify/vercel
- pacotes para iOS/Desktop/etc

## Características

Atualmente, a web suporta:

- Pré-renderização/hidratação

## Eventos

Os eventos regulares na `crate` `html` funcionam bem na web.

## Passos Futuros

Certifique-se de ler o [Guia Dioxus](https://dioxuslabs.com/guide) se você ainda não leu!
