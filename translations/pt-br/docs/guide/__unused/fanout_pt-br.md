# Distribuição

Um dos padrões de gerenciamento de estado mais confiáveis em grandes aplicativos Dioxus é o `fan-out`. O padrão de distribuição (fan-out) é a maneira ideal de estruturar seu aplicativo para maximizar a reutilização de código, testabilidade e renderização determinística.

## A estrutura

Com `fan-out`, nossos componentes individuais na posição "folha" (leaf) do nosso `VirtualDom` são "puros", tornando-os facilmente reutilizáveis, testáveis e determinísticos. Por exemplo, a barra de título do nosso aplicativo pode ser um componente bastante complicado.

```rust
#[derive(Props, PartialEq)]
struct TitlebarProps {
    title: String,
    subtitle: String,
}

fn Titlebar(cx: Scope<TitlebarProps>) -> Element {
    cx.render(rsx!{
        div {
            class: "titlebar"
            h1 { "{cx.props.title}" }
            h1 { "{cx.props.subtitle}" }
        }
    })
}
```

Se usássemos um estado global como `use_context` ou `Fermi`, poderíamos ficar tentados a injetar nosso `use_read` diretamente no componente.

```rust
fn Titlebar(cx: Scope<TitlebarProps>) -> Element {
    let title = use_read(&cx, TITLE);
    let subtitle = use_read(&cx, SUBTITLE);

    cx.render(rsx!{/* ui */})
}
```

Para muitos aplicativos - este é um bom padrão, especialmente se o componente for único. No entanto, se quisermos reutilizar o componente fora deste aplicativo, começaremos a encontrar problemas em que nossos contextos não estão disponíveis.

## Distribuindo

Para permitir que nosso componente de barra de título seja usado em outros aplicativos, queremos elevar nossos `Atoms` para fora do componente de barra de título. Organizaríamos vários outros componentes nesta seção do aplicativo para compartilhar alguns do mesmo estado.

```rust
fn DocsiteTitlesection(cx: Scope) {
    let title = use_read(&cx, TITLE);
    let subtitle = use_read(&cx, SUBTITLE);

    let username = use_read(&cx, USERNAME);
    let points = use_read(&cx, POINTS);

    cx.render(rsx!{
        TitleBar { title: title, subtitle: subtitle }
        UserBar { username: username, points: points }
    })
}
```

Infelizmente, esse componente encapsulador (`wrapper`) específico não pode ser reutilizado em aplicativos. No entanto, porque simplesmente envolve outros elementos reais, não é necessário. Somos livres para reutilizar nossos componentes `TitleBar` e `UserBar` em aplicativos com facilidade. Também sabemos que esse componente em particular é muito eficiente porque o encapsulador não possui `props` e é sempre memoizado. As únicas vezes em que esse componente é renderizado novamente é quando qualquer um dos átomos muda.

Essa é a beleza do Dioxus - sempre sabemos onde nossos componentes provavelmente serão renderizados novamente. Nossos componentes `wrapper` podem facilmente evitar grandes re-renderizações simplesmente memoizando seus componentes. Este sistema pode não ser tão elegante ou preciso quanto os sistemas de sinal encontrados em bibliotecas como Sycamore ou SolidJS, mas é bastante ergonômico.
