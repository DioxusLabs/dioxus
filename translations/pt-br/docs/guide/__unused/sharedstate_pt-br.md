# Estado global

Se seu aplicativo finalmente ficou grande o suficiente para que a passagem de valores pela árvore acabe poluindo a intenção do seu código, talvez seja hora de mudar para o estado global.

No Dioxus, o estado global é compartilhado por meio da API de contexto. Este guia mostrará como usar a `Context` API para simplificar o gerenciamento de estado.

## Fornecendo Contexto e Consumindo Contexto

A maneira mais simples de recuperar o estado compartilhado em seu aplicativo é por meio da API `Context`. A `Context` API permite fornecer e consumir um item de estado entre dois componentes.

Sempre que um componente fornece um contexto, ele fica acessível a qualquer componente filho.

> Nota: os componentes pai não podem "alcançar" e consumir o estado abaixo de sua posição na árvore.

A terminologia aqui é importante: contexto `provide` significa que o componente irá expor o estado e contexto `consume` significa que o componente filho pode adquirir um handle para o estado acima dele.

Em vez de usar chaves ou `'static`, o Dioxus prefere o padrão `NewType` para procurar o estado pai. Isso significa que cada estado que você expõe como um contexto deve ser seu próprio tipo exclusivo.

Na prática, você terá um componente que expõe algum estado:

```rust
#[derive(Clone)]
struct Title(String);

fn app(cx: Scope) -> Element {
    cx.use_hook(|_| {
        cx.provide_context(Title("Hello".to_string()));
    });

    cx.render(rsx!{
        Child {}
    })
}
```

E então em nosso componente, podemos consumir esse estado a qualquer momento:

```rust
fn Child(cx: Scope) -> Element {
    let name = cx.consume_context::<Title>();

    //
}
```

> Nota: chamar o estado "consumir" pode ser uma operação bastante cara computacionalmente para ser executada durante cada renderização. Prefira consumir o estado dentro de um `use_hook`:

```rust
fn Child(cx: Scope) -> Element {
    // cache our "consume_context" operation
    let name = cx.use_hook(|_| cx.consume_context::<Title>());
}
```

Todo `Context` deve ser clonado - o item será clonado em cada chamada de `consume_context`. Para tornar essa operação mais leve, considere envolver seu tipo em um `Rc` ou `Arc`.

<!-- ## Coroutines

The `use_coroutine` hook  -->

<!-- # `use_context` and `use_context_provider`

These -->
