<div align="center">
  <h1>Dioxus</h1>
</div>

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/dioxus">
    <img src="https://img.shields.io/crates/v/dioxus.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/dioxus">
    <img src="https://img.shields.io/crates/d/dioxus.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs -->
  <a href="https://docs.rs/dioxus">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
  <!-- CI -->
  <a href="https://github.com/jkelleyrtp/dioxus/actions">
    <img src="https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg"
      alt="CI status" />
  </a>

  <!--Awesome -->
  <a href="https://github.com/dioxuslabs/awesome-dioxus">
    <img src="https://cdn.rawgit.com/sindresorhus/awesome/d7305f38d29fed78fa85652e3a63e154dd8e8829/media/badge.svg" alt="Awesome Page" />
  </a>
  <!-- Discord -->
  <a href="https://discord.gg/XgGxMSkvUM">
    <img src="https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square" alt="Discord Link" />
  </a>
</div>

<div align="center">
  <h3>
    <a href="https://dioxuslabs.com"> Website </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/example-projects"> Exemplos </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/guide/pt-br"> Guia </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/notes/README/ZH_CN.md"> 中文 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/translations/ja-jp/README.md"> 日本語 </a>
  </h3>
</div>

<br/>

> 💡 Este é um documento traduzido voluntariamente. Se você viu erros de tradução e/ou erros de digitação, entre em contato: [@amindWalker](https://github.com/amindWalker), [GMail](bhrochamail@gmail.com) ou **_melhor ainda, envie um PR_**.

<br>

**Dioxus** é um framework ergonômico para construir interfaces de forma portátil, rápida, escalável e robusta com a linguagem de programação Rust.

```rust
fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    })
}
```

O Dioxus pode ser usado para desenvolver aplicativos Web, Desktop, sites estáticos, TUI, LiveView e mais. O Dioxus é inteiramente agnóstico de renderizador e pode ser usado como uma plataforma para qualquer renderizador.

Se você conhece React, então você já conhece o Dioxus.

### Funções Únicas:

- Aplicativos Desktop rodam nativamente (sem ElectronJS!) em menos de 10 linhas de código.
- Incrivelmente ergonômico e um poderoso gerenciador de estados.
- Documentação compreensiva - guias e explicações ao apontar o mouse para todos os elementos HTML e eventos.
- Extremamente eficiente em memória - 0 alocações globais para componentes com estado-estável.
- Agendamento assíncrono de canais-múltiplos para suporte de `async` de primera-classe.
- E mais! Leia as [publicações de lançamento](https://dioxuslabs.com/blog/introducing-dioxus/).

## Começando com...

<table style="width:100%" align="center">
    <tr>
        <th><a href="https://dioxuslabs.com/guide/pt-br/">Tutorial</a></th>
        <th><a href="https://dioxuslabs.com/reference/web">Web</a></th>
        <th><a href="https://dioxuslabs.com/reference/desktop/">Desktop</a></th>
        <th><a href="https://dioxuslabs.com/reference/ssr/">SSR</a></th>
        <th><a href="https://dioxuslabs.com/reference/mobile/">Móvel</a></th>
    <tr>
</table>

## Projetos de Exemplo:

| Navegador de Arquivos (Desktop)                                                                                                                                                          | WiFi Scanner (Desktop)                                                                                                                                                                 | TodoMVC (Todas as Plataformas)                                                                                                                                          | E-commerce com Tailwind (SSR/LiveView)                                                                                                                                                   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [![Explorador de Arquivos](https://github.com/DioxusLabs/example-projects/raw/master/file-explorer/image.png)](https://github.com/DioxusLabs/example-projects/blob/master/file-explorer) | [![Wifi Scanner Demo](https://github.com/DioxusLabs/example-projects/raw/master/wifi-scanner/demo_small.png)](https://github.com/DioxusLabs/example-projects/blob/master/wifi-scanner) | [![TodoMVC example](https://github.com/DioxusLabs/example-projects/raw/master/todomvc/example.png)](https://github.com/DioxusLabs/example-projects/blob/master/todomvc) | [![Exemplo de E-commerce](https://github.com/DioxusLabs/example-projects/raw/master/ecommerce-site/demo.png)](https://github.com/DioxusLabs/example-projects/blob/master/ecommerce-site) |

Veja a página [awesome-dioxus](https://github.com/DioxusLabs/awesome-dioxus) para uma lista curada do conteúdo dentro do ecossistema do Dioxus.

## Porquê o Dioxus e porquê o Rust?

TypeScript é uma adição fantástica ao JavaScript, mas ainda é fundamentalmente JavaScript. TS executa ligeiramente mais devagar, tem várias opções de configurações diferentes e nem todos os pacotes estão propriamente tipados.

Apenas por usar o Rust, nós ganhamos:

- Tipos estáticos para _todas_ as bibliotecas por padrão
- Imutabilidade por padrão
- Um sistema de módulos simples e intuitivo
- Documentação integrada na própria linguagem (`go to source` _de fato vai até a fonte_)
- Padrões de combinação avançados
- Iteradores limpos, eficientes e combináveis
- Testes de Unidade/Integração em linha integrados à linguagem
- O melhor da classe em Tratamento de Erros
- Biblioteca padrão poderosa e sensata
- Sistema de macros flexível
- Acesso ao `crates.io`

Especificamente, o Dioxus providencia para nós muitas outras garantias:

- Estrutura de Dados imutável apropriada
- Garantias para Tratamento de Erros (assim você pode dormir em paz à noite sem se preocupar com erros do tipo `undefined`)
- Desempenho móvel nativo
- Acesso direto ao sistema de Entrada/Saída (IO)

E muito mais. Dioxus faz com que aplicativos em Rust sejam rápidos de escrever como os de React, mas permite mais robustez dando ao sua equipe frontend mais confiança em desenvolver grandes mudanças em pouco tempo.

## Porquê não o Dioxus?

Você não deve usar o Dioxus se:

- Você não gosta da metodologia do React Hooks para o frontend
- Você precisa de um renderizador personalizado
- Você quer suporte para navegadores que não tenham suporte ao WASM ou asm.js
- Você precisa de uma solução `Send` + `Sync` (o Dioxus ainda não é `thread-safe`)

## Comparação com outras frameworks de UI em Rust:

Dioxus primeiramente enfatiza a **experiência do desenvolvedor** e a **familiaridade com os princípios do React**.

- [Yew](https://github.com/yewstack/yew): prefere o padrão `elm`, não há `props` emprestadas, suporta SSR (sem `hydration`), sem suporte direto para Desktop/Móvel.
- [Percy](https://github.com/chinedufn/percy): suporta SSR, mas com menos ênfase em gerenciamento de estado e tratamento de eventos.
- [Sycamore](https://github.com/sycamore-rs/sycamore): sem `VirtualDOM` usando controle preciso de reatividade, mas sem suporte direto à aplicativos Desktop/Móvel.
- [Dominator](https://github.com/Pauan/rust-dominator): alternativa zero-custo baseada em sinais, menos ênfase em comunidade e documentação.
- [Azul](https://azul.rs): renderizador HTML/CSS totalmente nativo para aplicações Desktop, sem suporte para Web/SSR.

## Paridade com React e Progresso

Dioxus é fortemente inspirado pelo React, mas nós queremos que sua transição pareça como um aprimoramento. Dioxus está _quase_ lá, mas ainda faltam alguma funções chave. Isto inclui:

- Portais
- Suspensão integrada ao SSR
- Componentes de Servidor / Segmentador de Pacotes / Execução Tardia (Lazy)

Dioxus é único no ecossistema do Rust por suportar:

- Componentes com propriedade que são emprestadas dos seus parentes
- SSR com `hydration` feito pelo Cliente
- Suporte à aplicação Desktop

Para mais informações sobre quais funções estão atualmente disponíveis e para o progresso futuro, veja [O Guia](https://dioxuslabs.com/guide/pt-br/).

## Projeto dentro do ecossistema Dioxus

Quer adentrar e ajudar a construir o futuro do frontend em Rust? Há um vasto número de lugares em que você pode contribuir e fazer uma grande diferença:

- [TUI renderer](https://github.com/dioxusLabs/plasmo)
- [Ferramentas CLI](https://github.com/dioxusLabs/cli)
- [Documentação e Exemplos de Projeto](https://github.com/dioxusLabs/docsite)
- LiveView e Servidor Web
- Sistema de Componentes prontos

## Licença

Este projeto é licenciado sob a licença MIT.

[licença mit]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

### Contribuições

A menos que você explicitamente ateste o contrário, qualquer contribuição feita ao Dioxus por você será licenciada de acordo com a licença MIT sem nenhum outro termo ou condição.
