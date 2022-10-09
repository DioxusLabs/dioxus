# Introdução

![dioxuslogo](./images/dioxuslogo_full.png)

Dioxus é uma estrutura portátil, de alto desempenho e ergonômica para a construção de interfaces de usuário multiplataforma no Rust. Este guia irá ajudá-lo a começar a escrever aplicativos Dioxus para a Web, Desktop, Mobile e muito mais.

```rust
fn app(cx: Scope) -> Element {
    let mut count = use_state(&cx, || 0);

    cx.render(rsx!(
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    ))
}
```

Dioxus é fortemente inspirado pelo React. Se você conhece o React, começar com o Dioxus será muito fácil.

> Este guia pressupõe que você já conhece um pouco de [Rust](https://www.rust-lang.org/)! Caso contrário, recomendamos ler [_the book_](https://doc.rust-lang.org/book/ch01-00-getting-started.html) para aprender Rust primeiro.

## Recursos

- Aplicativos de desktop rodando nativamente (sem Electron!) em menos de 10 linhas de código.
- Gerenciamento de estado incrivelmente ergonômico e poderoso.
- Documentação em linha abrangente - documentação sob o mouse e guias para todos os elementos HTML e manipuladores de eventos.
- Extremamente eficiente de memória – 0 alocações globais para componentes de estado estacionário.
- Agendador assíncrono multicanal para suporte assíncrono de primeira classe.
- E mais! Leia as [notas de lançamento completa](https://dioxuslabs.com/blog/introducing-dioxus/.

### Multi Plataforma

Dioxus é um kit de ferramentas _portátil_, o que significa que a implementação do núcleo pode ser executada em qualquer lugar sem independente da plataforma. Ao contrário de muitos outros kits de ferramentas de frontend Rust, o Dioxus não está intrinsecamente vinculado ao WebSys. Na verdade, todos os elementos e ouvintes de eventos podem ser trocados em tempo de compilação. Por padrão, o Dioxus vem com o recurso `html` habilitado, mas isso pode ser desabilitado dependendo do seu renderizador de destino.

No momento, temos vários renderizadores de terceiros:

- WebSys (para WASM): Ótimo suporte
- Tao/Tokio (para aplicativos de desktop): Bom suporte
- Tao/Tokio (para aplicativos móveis): Suporte ruim
- SSR (para gerar marcação estática)
- TUI/Rink (para aplicativos baseados em terminal): Experimental

## Estabilidade

Dioxus ainda não chegou a uma versão estável.

Web: como a Web é uma plataforma bastante madura, esperamos que haja muito pouca rotatividade de API para recursos baseados na Web.

Desktop: APIs provavelmente estarão em fluxo à medida que descobrimos padrões melhores do que nossa contraparte, ElectronJS.

SSR: Não esperamos que a API SSR mude drasticamente no futuro.
