# Elevação de Estados e Distribuição

Manter o estado local para os componentes nem sempre funciona.

Um dos padrões de gerenciamento de estado mais confiáveis em grandes aplicativos Dioxus é `lift-up` e `fan-out`. Elevar e distribuir o estado é a maneira ideal de estruturar seu aplicativo para maximizar a reutilização de código, testabilidade e renderização determinística.

## Elevando o Estado

Ao criar aplicativos complexos com o Dioxus, a melhor abordagem é começar colocando seu estado e uma interface do usuário em um único componente. Uma vez que seu componente tenha passado por algumas centenas de linhas, pode valer a pena refatorá-lo em alguns componentes menores.

Aqui, agora somos desafiados a compartilhar o estado entre esses vários componentes.

Digamos que refatoramos nosso componente para separar uma entrada e uma exibição.

```rust
fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        Title {}
        Input {}
    })
}
```

Sempre que um valor é inserido em nosso componente `Input`, precisamos de alguma forma propagar essas alterações no componente `Title`.

Uma solução rápida - que funciona para muitos aplicativos - é simplesmente compartilhar um `UseState` entre os dois componentes.

```rust
fn app(cx: Scope) -> Element {
    let text = use_state(&cx, || "default".to_string());

    cx.render(rsx!{
        Title { text: text.clone() }
        Input { text: text.clone() }
    })
}
```

> Nota: como nós usamos `Clone` no nosso handle `text` `UseState`, ambos `Title` e `Input` serão memoizados.

Aqui, nós "retiramos" o estado do nosso componente `Input` e o levamos para o ancestral compartilhado mais próximo. Em nosso componente de entrada, podemos usar diretamente esse handle `UseState` como se tivesse sido definido localmente:

```rust
#[inline_props]
fn Input(cx: Scope, text: UseState<String>) -> Element {
    cx.render(rsx!{
        input { oninput: move |evt| text.set(evt.value.clone()) }
    })
}
```

Da mesma forma, nosso componente `Title` seria direto:

```rust
#[inline_props]
fn Title(cx: Scope, text: UseState<String>) -> Element {
    cx.render(rsx!{
        h1 { "{text}" }
    })
}
```

Para casos de uso mais complicados, podemos aproveitar a coerção do `EventHandler` mencionada anteriormente para passar qualquer retorno. Lembre-se de que os campos em componentes que começam com "on" são "atualizados" automaticamente para um `EventHandler` no lugar da chamada.

Isso nos permite abstrair sobre o tipo exato de estado que está sendo usado para armazenar os dados.

Para o componente `Input`, simplesmente adicionaríamos um novo campo `oninput`:

```rust
#[inline_props]
fn Input<'a>(cx: Scope<'a>, oninput: EventHandler<'a, String>) -> Element {
    cx.render(rsx!{
        input { oninput: move |evt| oninput.call(evt.value.clone()), }
    })
}
```

Para o nosso componente `Title`, também podemos abstraí-lo para pegar qualquer `&str`:

```rust
#[inline_props]
fn Title<'a>(cx: Scope<'a>, text: &'a str) -> Element {
    cx.render(rsx!{
        h1 { "{text}" }
    })
}
```

## Distribuindo

À medida que seu aplicativo cresce e cresce, pode ser necessário começar a usar o estado global para evitar a propagração de `props`. Isso tende a resolver muitos problemas, mas gera outros ainda mais.

Por exemplo, digamos que construímos um belo componente `Title`. Em vez de passar `props`, estamos usando um hooks `use_read` do `Fermi`.

```rust
fn Title(cx: Scope) -> Element {
    let title = use_read(&cx, TITLE);

    cx.render(rsx!{
        h1 { "{title}" }
    })
}
```

Isso é ótimo - tudo está bem no mundo. Obtemos atualizações precisas, memoização automática e uma abstração sólida. Mas, o que acontece quando queremos reutilizar esse componente em outro projeto? Esse componente agora está profundamente entrelaçado com nosso estado global - que pode não ser o mesmo em outro aplicativo.

Nesse caso, queremos "retirar" nosso estado global dos componentes de "visualização". Com `lifting`, nossos componentes individuais na posição "folha" do nosso `VirtualDom` são "puros", tornando-os facilmente reutilizáveis, testáveis e determinísticos.

Para permitir que nosso componente de título seja usado em aplicativos, queremos elevar nossos átomos para fora do componente de título. Organizaríamos vários outros componentes nesta seção do aplicativo para compartilhar alguns do mesmo estado.

```rust
fn DocsiteTitlesection(cx: Scope) {
    // Use our global state in a wrapper component
    let title = use_read(&cx, TITLE);
    let subtitle = use_read(&cx, SUBTITLE);

    let username = use_read(&cx, USERNAME);
    let points = use_read(&cx, POINTS);

    // and then pass our global state in from the outside
    cx.render(rsx!{
        Title { title: title.clone(), subtitle: subtitle.clone() }
        User { username: username.clone(), points: points.clone() }
    })
}
```

Infelizmente, esse componente encapsulador (`wrapper`) específico não pode ser reutilizado em aplicativos. No entanto, porque simplesmente envolve outros elementos reais, não é necessário. Somos livres para reutilizar nossos componentes `TitleBar` e `UserBar` em aplicativos com facilidade. Também sabemos que esse componente em particular é muito eficiente porque o wrapper não possui props e é sempre memorizado. As únicas vezes em que esse componente é renderizado novamente é quando qualquer um dos átomos muda.

Essa é a beleza do Dioxus - sempre sabemos onde nossos componentes provavelmente serão renderizados novamente. Nossos componentes wrapper podem facilmente evitar grandes re-renderizações simplesmente memoizando seus componentes. Este sistema pode não ser tão elegante ou preciso quanto os sistemas de sinal encontrados em bibliotecas como `Sycamore` ou `SolidJS`, mas é bastante ergonômico.
