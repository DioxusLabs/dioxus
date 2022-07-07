# Atributos Especiais

Dioxus se esforça ao máximo para ser bem próximo do React, mas existem algumas divergências e "comportamentos especiais" que você deve revisar antes de seguir em frente.

Nesta seção, abordaremos atributos especiais incorporados ao Dioxus:

- `dangerous_inner_html`
- Atributos booleanos
- `prevent_default`
- manipuladores de eventos como atributos de string
- `value`, `checked` e `selected`

## A Escotilha de escape HTML: `dangerous_inner_html`

Uma coisa que você pode ter perdido no React é a capacidade de renderizar HTML bruto diretamente para o DOM. Se você estiver trabalhando com items pré-renderizados, saída de modelos ou saída de uma biblioteca JS, convém passar o HTML diretamente em vez de passar pelo Dioxus. Nesses casos, use `dangerous_inner_html`.

Por exemplo, enviar um conversor de markdown para Dioxus pode aumentar significativamente o tamanho final do aplicativo. Em vez disso, você poderia pré-renderizar sua remarcação para HTML e, em seguida, incluir o HTML diretamente em sua saída. Usamos essa abordagem para a [página inicial do Dioxus](https://dioxuslabs.com):

```rust
fn BlogPost(cx: Scope) -> Element {
    let contents = include_str!("../post.html");
    cx.render(rsx!{
        div {
            class: "markdown",
            dangerous_inner_html: "{contents}",
        }
    })
}
```

> Nota! Esse atributo é chamado de "dangerous_inner_html" porque é **perigoso** passar dados que você não confia. Se você não for cuidadoso, poderá se expor facilmente à ataques de scripts entre sites (XSS) para seus usuários.
>
> Se você estiver lidando com entradas não confiáveis, certifique-se de higienizar seu HTML antes de passá-lo para `dangerous_inner_html` – ou apenas passá-lo para um elemento de texto para escapar de qualquer tag HTML.

## Atributos booleanos

A maioria dos atributos, quando renderizados, serão renderizados exatamente como a entrada que você forneceu. No entanto, alguns atributos são considerados atributos "booleanos" e apenas sua presença determina se eles afetam ou não a saída. Para esses atributos, um valor fornecido de `"false"` fará com que eles sejam removidos do elemento de destino.

Então este RSX:

```rust
rsx!{
    div {
        hidden: "false",
        "hello"
    }
}
```

não renderizaria o atributo `hidden`:

```html
<div>hello</div>
```

No entanto, nem todos os atributos funcionam assim. _Apenas os seguintes atributos_ têm este comportamento:

- `allowfullscreen`
- `allowpaymentrequest`
- `async`
- `autofocus`
- `autoplay`
- `checked`
- `controls`
- `default`
- `defer`
- `disabled`
- `formnovalidate`
- `hidden`
- `ismap`
- `itemscope`
- `loop`
- `multiple`
- `muted`
- `nomodule`
- `novalidate`
- `open`
- `playsinline`
- `readonly`
- `required`
- `reversed`
- `selected`
- `truespeed`

Para quaisquer outros atributos, um valor de `"false"` será enviado diretamente para o DOM.

## Parando a entrada e a navegação do formulário com `prevent_default`

Atualmente, não é possível chamar `prevent_default` em `EventHandlers` no Desktop/Móvel. Até que isso seja suportado, é possível evitar o padrão usando o atributo `prevent_default`.

> Nota: você não pode impedir o padrão condicionalmente com esta abordagem. Esta é uma limitação até que o tratamento de eventos síncrono esteja disponível no limite do Webview

Para usar `prevent_default`, simplesmente anexe o atributo `prevent_default` a um determinado elemento e defina-o com o nome do manipulador de eventos no qual você deseja evitar o padrão. Podemos anexar esse atributo várias vezes para vários atributos.

```rust
rsx!{
    input {
        oninput: move |_| {},
        prevent_default: "oninput",

        onclick: move |_| {},
        prevent_default: "onclick",
    }
}
```

<!--
## Passing attributes into children: `..Attributes`

> Note: this is an experimental, unstable feature not available in released versions of Dioxus. Feel free to skip this section.

Just like Dioxus supports spreading component props into components, we also support spreading attributes into elements. This lets you pass any arbitrary attributes through components into elements.


```rust
#[derive(Props)]
pub struct InputProps<'a> {
    pub children: Element<'a>,
    pub attributes: Attribute<'a>
}

pub fn StateInput<'a>(cx: Scope<'a, InputProps<'a>>) -> Element {
    cx.render(rsx! (
        input {
            ..cx.props.attributes,
            &cx.props.children,
        }
    ))
}
``` -->

## Entradas Controladas e `value`, `checked` e `selected`

No Dioxus, há uma distinção entre entradas controladas e não controladas. A maioria das entradas que você usará são controladas, o que significa que ambos direcionamos o `valor` da entrada e reagimos à `oninput`.

Componentes controlados:

```rust
let value = use_state(&cx, || String::from("hello world"));

rsx! {
    input {
        oninput: move |evt| value.set(evt.value.clone()),
        value: "{value}",
    }
}
```

Com entradas não controladas, não iremos realmente direcionar o valor do componente. Isso tem suas vantagens quando não queremos renderizar novamente o componente quando o usuário insere um valor. Poderíamos selecionar o elemento diretamente - algo que o Dioxus não suporta em todas as plataformas - ou poderíamos manipular `oninput` e modificar um valor sem causar uma atualização:

```rust
let value = use_ref(&cx, || String::from("hello world"));

rsx! {
    input {
        oninput: move |evt| *value.write_silent() = evt.value.clone(),
        // no "value" is driven here – the input keeps track of its own value, and you can't change it
    }
}
```

## Strings para manipuladores (`handlers`) como `onclick`

Para campos de elemento que recebem um manipulador como `onclick` ou `oninput`, o Dioxus permite anexar um encerramento. Como alternativa, você também pode passar uma string usando a sintaxe de atributo normal e atribuir esse atributo no DOM.

Isso permite que você use JavaScript (somente se seu renderizador puder executar JavaScript).

```rust
rsx!{
    div {
        // handle oninput with rust
        oninput: move |_| {},

        // or handle oninput with javascript
        oninput: "alert('hello world')",
    }
}

```

## Recapitulando

Neste capítulo, aprendemos:

- Como declarar elementos
- Como renderizar condicionalmente partes da sua interface do usuário
- Como renderizar listas
- Quais atributos são "especiais"

<!-- todo
There's more to elements! For further reading, check out:

- [Custom Elements]()
-->
