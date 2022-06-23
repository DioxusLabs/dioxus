# Introdução

![dioxuslogo](./images/dioxuslogo_full.png)

**Dioxus** é uma biblioteca para construir interfaces de forma rápida, escalável e robusta com a linguagem de programação Rust. Este guia ajudará você a começar com o Dioxus para Web, Desktop, Móvel e mais.

```rust
fn app(cx: Scope) -> Element {
    let mut count = use_state(&cx, || 0);

    cx.render(rsx!(
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    ))
};
```

Em geral, Dioxus e React compartilham muitas funcionalidades similares. Se este guia estiver faltando qualquer conceito ou alguma messagem de error estiver confusa, a documentação de React pode ser mais útil. Nós estamos nos dedicando a prover uma ferramenta _familiar_ para interfaces em Rust, então nós escolhemos seguir os passos de UI populares (React, Redux, etc). Se você conhece React, então você já conhece o Dioxus. Se você não conhece nenhum, este guia irá te ajudar!

> Este é um livro de introdução! Para tópicos avançados veja: [Reference](/reference).

## Multi-platforma

Dioxus é uma ferramenta _portátil_, significando que implementações do Núcleo podem executar em qualquer lugar sem depender da plataforma. Diferente de muitas outras ferramentas para frontend em Rust, Dioxus não está intrinsecamente ligado ao `WebSys`. Na verdade, cada elemento e ouvinte de eventos podem ser trocados na hora de compilar. Por padrão, Dioxus vem com o `html` macro habilitado, mas isto pode ser desabilitado dependendo do seu objetivo de renderização.

Atualmente, nós temos vários renderizadores de primeira-mão:
Right now, we have several 1st-party renderers:

- WebSys (for WASM)
- Tao/Tokio (for Desktop apps)
- Tao/Tokio (for Mobile apps)
- SSR (for generating static markup)
- TUI/Rink (for terminal-based apps)

### Suporte Web

---

A Web é a plataforma com mais suporte. Para executar na Web, o seu aplicativo deve ser compilado para `WebAssembly` e depende do pacote `dioxus` com a função `web` ativa. Por conta das limitações do `WASM` nem todos os pacotes (`crates`) podem funcionar com seu aplicativo-web, então você precisará ter certeza que os pacotes que você usa funcionam sem chamadas de sistema (timers, IO, etc).

Por conta da Web ser uma plataforma bastante madura, nós esperamos pouca fricção de APIs para funções baseadas em Web.

[Pular para o Guia: Começando para Web.](/reference/platforms/web)

Exemplos:

- [TodoMVC](https://github.com/DioxusLabs/example-projects/tree/master/todomvc)
- [ECommerce](https://github.com/DioxusLabs/example-projects/tree/master/ecommerce-site)

[![TodoMVC examplo](https://github.com/DioxusLabs/example-projects/raw/master/todomvc/example.png)](https://github.com/DioxusLabs/example-projects/blob/master/todomvc)

### Suporte SSR

---

Dioxus suporta renderização executada pelo lado do servidor!

Para renderizar estaticamente para um arquivo `.html` ou de um `WebServer`, então você precisará ter certeza que a função `ssr` está habilitada no `dioxus` e então usar `dioxus::ssr` API. Nós não esperamos que a API do SSR mude drasticamente no futuro.

```rust
let contents = dioxus::ssr::render_vdom(&dom);
```

[Pular para o Guia: Começando com SSR.](/reference/platforms/ssr)

Exemplos:

- [Exemplo de DocSite](https://github.com/dioxusLabs/docsite)
- [Tide WebServer]()
- [Markdown to fancy HTML generator]()

### Supporte para Desktop

---

A plataforma Desktop do Dioxus é poderosa, mas ainda limitada em funcionalidade quando comparada à plataforma Web. Poe enquanto, aplicativos de Desktop são renderizados pela biblioteca de WebView da plataforma, mas o seu código Rust é executado nativamente como uma tarefa nativa do sistema. Isso significa que APIs do navegador Web não está disponíveis, então renderizar WebGL, Canvas, etc não é uma tarefa fácil como é na Web. No entanto, chamadas nativas do sistema estão acessíveis, então transmissão em tempo-real, WebSockets, sistema de arquivos, etc são todas chamadas de API válidas. No futuro, nós planejamos mudar para uma renderizador `DOM` personalizado baseado em `webrender` com integração ao `WGPU`.

APIs de Desktop provavelmente estarão em fluxo enquanto descobrimos uma melhor solução que nosso padrão em ElectronJS.

[Pular para o Guia: Começando para Desktop.](/reference/platforms/desktop)

Examplos:

- [Explorador de Arquivos](https://github.com/DioxusLabs/example-projects/blob/master/file-explorer)
- [WiFi scanner](https://github.com/DioxusLabs/example-projects/blob/master/wifi-scanner)

[![Exemplo de Explorador de Arquivos](https://raw.githubusercontent.com/DioxusLabs/example-projects/master/file-explorer/image.png)](https://github.com/DioxusLabs/example-projects/tree/master/file-explorer)

### Suporte Móvel

---

A plataforma móvel é atualmente a menos suportada pelo Dioxus. Aplicativos móveis são renderizados com a WebView da plataforma, significando que animações, transparências, widgets natives não estão disponíveis. Além do mais, iOS é a única plataforma suportada. É possível executar o Dioxus no Android e renderizar com uma WebView, mas a biblioteca de janelas do Rust que o Dioxus usa - Tao - ainda não suporta o Android.

O suporte móvel é atualmente melhor destinado para aplicativos do tipo CRUD, idealmente para equipes que precisem desenvolver rapidamente, mas não se importam com animações ou widgets nativos.

[Pular para o Guia: Começando para Móvel.](/reference/platforms/mobile)

Exemplos:

- [Todo App](https://github.com/DioxusLabs/example-projects/blob/master/ios_demo)

### LiveView / Suporte de Componente de Servidor

---

A arquitetura interna do Dioxus foi projetada desde o dia 1 para suportar `LiveView`, onde um servidor hospeda um aplicativo para cada usuário conectado. Até hoje, ainda não temos suporte de primeira-classe ao `LiveView` - você terá que ligar o código você mesmo.

Enquanto não estiver totalmente implementado, a expectativa é de que aplicativos `LiveView` possam ser híbridos entre `WASM` e `SSR` onde apenas porções da página estão "live" enquanto o resto da página está em `SSR`, estaticamente gerada ou gerenciada pelo SPA cliente.

### Suporte a Multi-processadores

---

A `VirtualDOM` do Dioxus, infelizmente, não é atualmente `Send`. Internamente, nós usamos uma quantia de mutabilidade interna, o que não considera tarefa-segura. Isso significa que você não pode facilmente usar o Dioxus como a maioria dos Frameworks como Tide, Rocket, Axum, etc.

Para resolver isso, você terá que gerar a `VirtualDOM` no seu próprio processo e comunicar com ela através de `channels`.

Quando estiver trabalhando com Web Frameworks que requerem `Send`, é possível renderizar uma `VirtualDOM` imediatamente para uma `String` - mas você não pode reter a `VirtualDOM` usando um `await`. Para estados de SSR com retenção/pendentes (essencialmente `LiveView`) você precisará criar uma `pool` de processos de `VirtualDOM`.

Finalmente, você pode sempre encapsular a `VirtualDOM` com um tipo `Send` e manualmente providenciar as garantias `Send` você mesmo.
