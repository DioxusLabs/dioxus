# Memoization e a Arena de Alocações

O Dioxus difere ligeiramente de outras interfaces com `VirtualDOM` por ter uma maneira sutil de utilizar o alocador de memória.

Um aspecto importante para se entender é como as propriedades são passadas abaixo dos componentes pais para os filhos.

Todos os componentes (criados pelos elementos de interface) são firmemente alocados juntos em uma "arena".

No entanto, por conta das propriedades serem genericamente tipadas, eles são elegidos ao tipo `Any` e alocados na região `Heap` da memória - não dentro da mesma arena dos componentes.

Com esse sistema, nós tentamos ser mais eficientes quando deixamos o componente da arena e entramos na região `Heap`. Por padrão, as propriedades serão ["memoizadas"](https://en.wikipedia.org/wiki/Memoization) entre as renderizações usando `COW` (Copy Over Write) e `Context`.

Isso faz com que as propriedades sejam rapidamente comparadas - feito através de ponteiras de comparação (`ptr`) na ponteira `COW`.

Devido a essa `memoization` feita por padrão, o componente pai _não_ irá propagar uma re-renderização para os filhos se as propriedades dos filhos não mudarem.

https://dmitripavlutin.com/use-react-memo-wisely/

Este comportamente é defino pelos atributos implícitos ao usuário dos componentes.

Enquanto no mundo do React você encapsularia um componente com `react.memo`, no Dioxus os componentes são memoizados automaticamente via um atributo implícito.

Você pode configurar manualmente este comportamento em qualquer componente com a função `nomemo` para desabilitar a função de memoize.

```rust
fn test() -> DomTree {
    html! {
        <>
            <SomeComponent nomemo />
            // same as
            <SomeComponent nomemo=true />
        </>
    }
}

static TestComponent: Component = |cx| html!{<div>"Hello world"</div>};

static TestComponent: Component = |cx|{
    let g = "BLAH";
    html! {
        <div> "Hello world" </div>
    }
};

#[inline_props]
fn test_component(cx: Scope, name: String) -> Element {
    rsx!(cx, "Hello, {name}")
}
```

## Por este comportamento?

"Isto é diferente do React, porquê essa diferença?"

Pegue um componente como este:

```rust
fn test(cx: Scope) -> DomTree {
    let Bundle { alpha, beta, gamma } = use_context::<SomeContext>(cx);
    html! {
        <div>
            <Component name=alpha />
            <Component name=beta />
            <Component name=gamma />
        </div>
    }
}
```

Enquanto o conteúdo da de-estruturação do `Bundle` pode mudar, nem todos os filhos do componente irão mudar para serem re-renderizados toda vez que o `Context` mudar.
